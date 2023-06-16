#![allow(dead_code)]
use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use matrix_sdk::ruma::events::room::message::{MessageFormat, TextMessageEventContent};
use matrix_sdk::ruma::{
    EventId, OwnedEventId, OwnedRoomAliasId, OwnedRoomId, OwnedRoomOrAliasId, OwnedUserId,
    RoomAliasId, RoomId, UserId,
};
use matrix_sdk::Client;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Cmd {
    parts: Vec<CmdPart>,
    pointer: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CmdPart {
    Word(String),
    Quote,
    UserId(OwnedUserId),
    RoomId(OwnedRoomId),
    RoomAlias(OwnedRoomAliasId),
    EventId(EventRoom, OwnedEventId),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EventRoom {
    Id(OwnedRoomId),
    Alias(OwnedRoomAliasId),
}

fn parse_plain_text(input: &str) -> anyhow::Result<Vec<CmdPart>> {
    let mut cmd_parts = Vec::new();
    for word in input.split(' ') {
        if word.is_empty() {
            continue;
        }
        match parse_url(word) {
            Some(part) => cmd_parts.push(part),
            None => {
                if let Ok(v) = UserId::parse(word) {
                    cmd_parts.push(CmdPart::UserId(v));
                    continue;
                }
                if let Ok(v) = RoomId::parse(word) {
                    cmd_parts.push(CmdPart::RoomId(v));
                    continue;
                }
                if let Ok(v) = RoomAliasId::parse(word) {
                    cmd_parts.push(CmdPart::RoomAlias(v));
                    continue;
                }
                let word = html_escape::decode_html_entities(word);
                let mut quote_iter = word.split('"');
                if let Some(first_word) = quote_iter.next() {
                    if !first_word.is_empty() {
                        cmd_parts.push(CmdPart::Word(first_word.to_owned()));
                    }
                }
                for word in quote_iter {
                    cmd_parts.push(CmdPart::Quote);
                    if !word.is_empty() {
                        cmd_parts.push(CmdPart::Word(word.to_owned()));
                    }
                }
            }
        }
    }
    Ok(cmd_parts)
}

fn parse_html(input: &str) -> anyhow::Result<Vec<CmdPart>> {
    let dom = html_parser::Dom::parse(input)?;
    let mut cmd_parts = Vec::new();
    for child in dom.children {
        match child {
            html_parser::Node::Text(text) => {
                cmd_parts.extend(parse_plain_text(text.as_str())?);
            }
            #[allow(clippy::single_match)] // might add more supported tags
            html_parser::Node::Element(element) => match element.name.as_str() {
                "a" => {
                    if let Some(Some(url)) = element.attributes.get("href") {
                        if let Some(part) = parse_url(url) {
                            cmd_parts.push(part);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(cmd_parts)
}

fn parse_url(url: &str) -> Option<CmdPart> {
    let url = percent_encoding::percent_decode_str(url)
        .decode_utf8()
        .ok()?
        .into_owned();
    let parts: Vec<&str> = url
        .strip_prefix("https://matrix.to/#/")?
        .split('?')
        .next()?
        .split('/')
        .collect();
    match parts.as_slice() {
        [user_id] if user_id.starts_with('@') => {
            return Some(CmdPart::UserId(UserId::parse(user_id).ok()?))
        }
        [room_alias] if room_alias.starts_with('#') => {
            return Some(CmdPart::RoomAlias(RoomAliasId::parse(room_alias).ok()?))
        }
        [room_id] if room_id.starts_with('!') => {
            return Some(CmdPart::RoomId(RoomId::parse(room_id).ok()?))
        }
        [room_id, event_id] if room_id.starts_with('!') && event_id.starts_with('$') => {
            return Some(CmdPart::EventId(
                EventRoom::Id(RoomId::parse(room_id).ok()?),
                EventId::parse(event_id).ok()?,
            ));
        }
        [room_alias, event_id] if room_alias.starts_with('#') && event_id.starts_with('$') => {
            return Some(CmdPart::EventId(
                EventRoom::Alias(RoomAliasId::parse(room_alias).ok()?),
                EventId::parse(event_id).ok()?,
            ));
        }
        _ => {}
    }
    None
}

macro_rules! get {
    ($self:expr) => {
        $self.parts.get($self.pointer)
    };
}

macro_rules! ret {
    ($self:expr, $v:expr) => {{
        $self.pointer += 1;
        Some($v)
    }};
}

impl Cmd {
    pub fn parse(input: &TextMessageEventContent) -> anyhow::Result<Self> {
        let cmd_parts = match &input.formatted {
            Some(formatted_body) if formatted_body.format == MessageFormat::Html => {
                parse_html(&formatted_body.body)?
            }
            _ => parse_plain_text(&input.body)?,
        };
        Ok(Cmd {
            parts: cmd_parts,
            pointer: 0,
        })
    }

    pub fn pop(&mut self) -> Option<CmdPart> {
        let v = self.parts.get(self.pointer).cloned();
        self.pointer += 1;
        v
    }

    pub fn peek(&self) -> Option<CmdPart> {
        self.parts.get(self.pointer).cloned()
    }

    pub fn as_slice(&self) -> &[CmdPart] {
        &self.parts[self.pointer..]
    }

    pub fn len(&self) -> usize {
        self.parts.len() - self.pointer
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pop_word(&mut self) -> Option<String> {
        match get!(self) {
            Some(CmdPart::Word(word)) => ret!(self, word.clone()),
            _ => None,
        }
    }

    pub fn pop_number<T: FromStr>(&mut self) -> Option<T> {
        match get!(self) {
            Some(CmdPart::Word(word)) => match word.parse::<T>() {
                Ok(num) => ret!(self, num),
                _ => None,
            },
            _ => None,
        }
    }

    pub async fn pop_room_id(&mut self, client: &Client) -> anyhow::Result<Option<OwnedRoomId>> {
        match get!(self) {
            Some(CmdPart::RoomId(id)) => {
                self.pointer += 1;
                Ok(Some(id.to_owned()))
            }
            Some(CmdPart::RoomAlias(alias)) => {
                self.pointer += 1;
                Ok(Some(
                    client
                        .resolve_room_alias(alias)
                        .await
                        .with_context(|| anyhow!("Error resolving room alias"))?
                        .room_id,
                ))
            }
            _ => Ok(None),
        }
    }

    pub fn pop_room_alias(&mut self) -> Option<OwnedRoomAliasId> {
        match get!(self) {
            Some(CmdPart::RoomAlias(alias)) => ret!(self, alias.to_owned()),
            _ => None,
        }
    }

    pub fn pop_room_id_or_alias(&mut self) -> Option<OwnedRoomOrAliasId> {
        match get!(self) {
            Some(CmdPart::RoomId(id)) => ret!(self, id.to_owned().into()),
            Some(CmdPart::RoomAlias(alias)) => ret!(self, alias.to_owned().into()),
            _ => None,
        }
    }

    pub fn pop_user_id(&mut self) -> Option<OwnedUserId> {
        match get!(self) {
            Some(CmdPart::UserId(id)) => ret!(self, id.to_owned()),
            _ => None,
        }
    }

    pub fn pop_quoted_string(&mut self) -> Option<String> {
        match get!(self) {
            Some(CmdPart::Quote) => {
                self.pointer += 1; // Skip Quote
                let mut output = String::new();
                let mut first = true;
                while let Some(part) = self.pop() {
                    if part == CmdPart::Quote {
                        break;
                    }
                    if first {
                        first = false;
                    } else {
                        output.push(' ');
                    }
                    output.push_str(&part.to_string());
                }
                Some(output)
            }
            Some(part) => Some(part.to_string()),
            None => None,
        }
    }

    pub fn pop_remaining_into_string(self) -> Option<String> {
        if get!(self).is_some() {
            Some(self.into_string())
        } else {
            None
        }
    }

    pub fn into_string(self) -> String {
        let mut output = String::new();
        let mut first = true;
        for part in &self.parts[self.pointer..] {
            if first {
                first = false;
            } else {
                output.push(' ');
            }
            output.push_str(&part.to_string());
        }
        output
    }
}

impl Display for CmdPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdPart::RoomAlias(alias) => alias.fmt(f),
            CmdPart::RoomId(id) => id.fmt(f),
            CmdPart::UserId(id) => id.fmt(f),
            CmdPart::Word(word) => word.fmt(f),
            CmdPart::Quote => write!(f, "\""),
            CmdPart::EventId(room, event) => write!(f, "https://matrix.to/#/{room}/{event}"),
        }
    }
}

impl EventRoom {
    pub async fn as_room_id(&self, client: &Client) -> anyhow::Result<OwnedRoomId> {
        match self {
            Self::Id(id) => Ok(id.clone()),
            Self::Alias(alias) => Ok(client
                .resolve_room_alias(alias)
                .await
                .with_context(|| anyhow!("Error resolving room alias"))?
                .room_id),
        }
    }
}

impl Display for EventRoom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventRoom::Id(id) => id.fmt(f),
            EventRoom::Alias(alias) => alias.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_html;
    use super::CmdPart::*;
    use super::EventRoom::*;

    #[test]
    fn test1() {
        let input = "test <a href=\"https://matrix.to/#/#emily-template-test-room:the-apothecary.club\">#emily-template-test-room:the-apothecary.club</a> <a href=\"https://matrix.to/#/@emily-dev:the-apothecary.club\">emily-dev</a> &quot;some quoted text&quot;  https://matrix.to/#/!JmUZDmdizioytNxcBx:the-apothecary.club/$_DY2gc3mXsj2nvSEUkxpOVPxAIDerLv9xcoaDgAk9Og?via=the-apothecary.club&amp;via=matrix.org";
        let expected = vec![
            Word("test".to_owned()),
            RoomAlias(
                "#emily-template-test-room:the-apothecary.club"
                    .try_into()
                    .unwrap(),
            ),
            UserId("@emily-dev:the-apothecary.club".try_into().unwrap()),
            Quote,
            Word("some".to_owned()),
            Word("quoted".to_owned()),
            Word("text".to_owned()),
            Quote,
            EventId(
                Id("!JmUZDmdizioytNxcBx:the-apothecary.club"
                    .try_into()
                    .unwrap()),
                "$_DY2gc3mXsj2nvSEUkxpOVPxAIDerLv9xcoaDgAk9Og"
                    .try_into()
                    .unwrap(),
            ),
        ];
        let cmd_parts = parse_html(input).unwrap();
        assert_eq!(cmd_parts, expected);
    }

    #[test]
    fn test2() {
        let input = "<a href=\"https://matrix.to/#/%23heavy-topics%3Athe-apothecary.club\">#heavy-topics:the-apothecary.club</a> test";
        let expected = vec![
            RoomAlias("#heavy-topics:the-apothecary.club".try_into().unwrap()),
            Word("test".to_owned()),
        ];
        let cmd_parts = parse_html(input).unwrap();
        assert_eq!(cmd_parts, expected);
    }

    #[test]
    fn test3() {
        let input = "<mx-reply><blockquote><a href=\"https://matrix.to/#/!JmUZDmdizioytNxcBx:the-apothecary.club/$fOt_uoyD9gzlperifUtfY3-tjBgUw4i3sCXTx7fNkGc?via=the-apothecary.club&via=matrix.org\">In reply to</a> <a href=\"https://matrix.to/#/@sasha:the-apothecary.club\">@sasha:the-apothecary.club</a><br><a href=\"https://matrix.to/#/%23heavy-topics%3Athe-apothecary.club\">#heavy-topics:the-apothecary.club</a> test</blockquote></mx-reply>and what does this do";
        let expected = vec![
            Word("and".to_owned()),
            Word("what".to_owned()),
            Word("does".to_owned()),
            Word("this".to_owned()),
            Word("do".to_owned()),
        ];
        let cmd_parts = parse_html(input).unwrap();
        assert_eq!(cmd_parts, expected);
    }

    #[test]
    fn test4() {
        let input = "<a href=\"https://matrix.to/#/%23heavy-topics%3Athe-apothecary.club\">#heavy-topics:the-apothecary.club</a> <a href=\"https://matrix.to/#/%21JmUZDmdizioytNxcBx%3Athe-apothecary.club/%2448Po10iXIYTSiXXkbtagpqpXlpQVsXba2ypFSjkKnsM?via=the-apothecary.club&amp;via=matrix.org\">https://matrix.to/#/%21JmUZDmdizioytNxcBx%3Athe-apothecary.club/%2448Po10iXIYTSiXXkbtagpqpXlpQVsXba2ypFSjkKnsM?via=the-apothecary.club&amp;via=matrix.org</a> meowww&quot;&quot;&quot;hello";
        let expected = vec![
            RoomAlias("#heavy-topics:the-apothecary.club".try_into().unwrap()),
            EventId(
                Id("!JmUZDmdizioytNxcBx:the-apothecary.club"
                    .try_into()
                    .unwrap()),
                "$48Po10iXIYTSiXXkbtagpqpXlpQVsXba2ypFSjkKnsM"
                    .try_into()
                    .unwrap(),
            ),
            Word("meowww".to_owned()),
            Quote,
            Quote,
            Quote,
            Word("hello".to_owned()),
        ];
        let cmd_parts = parse_html(input).unwrap();
        assert_eq!(cmd_parts, expected);
    }
}
