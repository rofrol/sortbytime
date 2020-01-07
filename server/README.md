## html exploration

`cargo install htmlq`

count: `htmlq -p "#itemsStream > li .diggbox span:first-of-type" < out.html`

title: `htmlq -p "#itemsStream > li:first-of-type h2 a" --text < out.html`

date_published: `htmlq -p '#itemsStream > li:first-of-type [itemprop="datePublished"]' --attribute datetime < out.html`

source: `htmlq -p '#itemsStream > li:first-of-type [title~="źródło"]' --attribute href  < out.html`

description: `htmlq -p '#itemsStream > li:first-of-type .description' --text < out.html`

## test request

`curl -H "Content-Type: application/json" "http://localhost:8088/hity" -d '{"id":"1", "name": "JohnDoe"}'`

`curl -s -H "Content-Type: application/json" "http://localhost:8088/hity" -d '{"id":"1", "name": "JohnDoe"}' | jq '.'`

## Run tests

Run all tests except ignored:

`cargo test`

Or single test

`cargo test get_items_test`

Run ignored tests

`cargo test -- --ignored`

To show println!

```bash
$ cargo test -- --nocapture
$ cargo test get_items_test -- --nocapture
```

with cargo watch:

`cargo watch -x 'test get_items_test'`
