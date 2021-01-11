This is an actix-web Json REST API server that uses sqlx to query postgres.

The purpose of this app is to reproduce a protocol error.

    [2021-01-11T00:41:05Z ERROR sqlx_actix_test::stream] stream pool: Protocol("unknown message type: \'\\u{0}\'")

To reproduce this, run the app and the test concurrently:

    cargo run --release &
    drill --stats -q --benchmark tests/local.yml

The errors seem to occur more consistently with:

* PgPoolOptions::test_before_acquire(false)
* lots of connections.
* more queries scheduled than available connections.
* a bulky json field or a very long (32KB) text field in the database
  record.  Json seems to trigger the issue more easily, and with a
  smaller record than a text field can.
* streaming data from sqlx to actix responses (see the /junkstream
  method).
  
