BEGIN;

INSERT INTO users
(mxid)
VALUES
('@test:test.local');

INSERT INTO identities
(mxid, name, display_name, activators)
VALUES
('@test:test.local', 'meow', 'Meow Kitty', '{"m"}'),
('@test:test.local', 'beep', 'Beep Boop', '{"b"}');

COMMIT;
