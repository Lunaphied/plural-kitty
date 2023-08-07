BEGIN;

INSERT INTO users
(mxid)
VALUES
('@test:test.local');

INSERT INTO members
(mxid, name, display_name, activators, track_account)
VALUES
('@test:test.local', 'meow', 'Meow Kitty', '{"m"}', false),
('@test:test.local', 'beep', 'Beep Boop', '{"b"}', false),
('@test:test.local', 'test', '', '{"t"}', true);

COMMIT;
