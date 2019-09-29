// This is a contrived example intended to illustrate actix-web features.
// *Imagine* that you have a process that involves 3 steps.  The steps here
// are dumb in that they do nothing other than call an
// httpbin endpoint that returns the json that was posted to it.  The intent
// here is to illustrate how to chain these steps together as futures and return
// a final result in a response.
//
// Actix-web features illustrated here include:
//     1. handling json input param
//     2. validating user-submitted parameters using the 'validator' crate
//     2. actix-web client features:
//           - POSTing json body
//     3. chaining futures into a single response used by an asynch endpoint

#[macro_use]
extern crate validator_derive;

#[macro_use]
extern crate serde_derive;

use std::io;

use actix_web::client::Client;
use actix_web::{web, App, Error as ActixError, HttpResponse, HttpServer};
use futures::Future;
use validator::Validate;

extern crate kuchiki;
use kuchiki::traits::*;
use kuchiki::NodeRef;
//use kuchiki::tree::{ElementData, Html, NodeDataRef, NodeRef};

use derive_more::Display;

extern crate chrono;
use chrono::{DateTime, Local};

#[derive(Debug, Validate, Deserialize, Serialize)]
struct SomeData {
    #[validate(length(min = 1, max = 1000000))]
    id: String,
    #[validate(length(min = 1, max = 100))]
    name: String,
}

fn hity(
    _some_data: web::Json<SomeData>,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = ActixError> {
    client
        .get("https://www.wykop.pl/hity/dnia/")
        .send()
        .map_err(ActixError::from)
        .and_then(|mut resp| {
            resp.body().from_err().and_then(|body| {
                let body_string = String::from_utf8(body.to_vec()).unwrap();
                let items = get_items(&body_string);
                println!("items: {:?}", items);

                Ok(HttpResponse::Ok().body("Hello"))
            })
        })
}

#[derive(Debug, PartialEq)]
struct Item {
    count: u32,
    title: String,
    href: String,
    date_published: DateTime<Local>,
    source: String,
    description: String,
    author: String,
    author_url: String,
}

#[derive(Debug, Display)]
pub enum MyError {
    #[display(fmt = "Missing item")]
    MissingItems,
}

fn get_items(html: &str) -> Result<Vec<Item>, MyError> {
    let css_selector = "#itemsStream li";

    // println!("{:?}", html);
    let document = kuchiki::parse_html().one(html);
    Ok(document
        .select(css_selector)
        .map_err(|()| MyError::MissingItems)?
        .take(1)
        .map(|item| {
            dbg!(&item);

            println!("{:?}", item.text_contents());

            let item_node: &kuchiki::NodeRef = item.as_node();

            let count_node_data_ref = item_node
                .select_first(".diggbox span:first-of-type")
                .unwrap();
            println!("count_node_data_ref: {:?}", count_node_data_ref);
            println!(
                "count_node_data_ref.text_contents(): {:?}",
                count_node_data_ref.text_contents()
            );
            let count: u32 = count_node_data_ref.text_contents().parse().unwrap();

            let a_node_data_ref = item_node.select_first("h2 a").unwrap();
            let a_node = a_node_data_ref.as_node();

            let title: String = a_node_data_ref.text_contents();
            println!("title: {}", title);

            let a_elt = a_node.as_element().unwrap();
            let href = a_elt.attributes.borrow().get("href").unwrap().to_string();
            println!("href: {:?}", href);

            let date_published_string = item_node
                .select_first("[itemprop='datePublished']")
                .unwrap()
                .as_node()
                .as_element()
                .unwrap()
                .attributes
                .borrow()
                .get("datetime")
                .unwrap()
                .to_string();

            let date_published = DateTime::parse_from_rfc3339(&date_published_string)
                .unwrap()
                .with_timezone(&Local);
            println!("date_published: {:?}", date_published);

            let source = item_node
                .select_first("[title~='źródło']")
                .unwrap()
                .as_node()
                .as_element()
                .unwrap()
                .attributes
                .borrow()
                .get("href")
                .unwrap()
                .to_string();

            println!("source: {:?}", source);

            let description = item_node
                .select_first(".description a")
                .unwrap()
                .as_node()
                .text_contents()
                .trim()
                .to_string();

            println!("description: {:?}", description);

            let author_node_data_ref = item_node.select_first("a[href*='ludzie'").unwrap();
            let author_node = author_node_data_ref.as_node();
            println!("author_node: {:?}", author_node);

            let author = author_node
                .text_contents()
                .trim()
                // TODO: Get only text node
                // https://stackoverflow.com/questions/56329121/how-to-get-only-text-node-with-kuchiki
                // https://users.rust-lang.org/t/how-to-get-only-text-node-with-kuchiki/29084
                .trim_start_matches('@')
                .to_string();

            println!("author: {:?}", author);

            let author_url = author_node
                .as_element()
                .unwrap()
                .attributes
                .borrow()
                .get("href")
                .unwrap()
                .to_string();

            println!("author_url: {:?}", author_url);

            let author_nodes: Vec<NodeRef> = author_node
                .descendants()
                .filter(|node| {
                    println!("node: {:?}", node);
                    true
                })
                .collect();

            println!("author_nodes: {:?}", author_nodes);

            Item {
                count,
                title,
                href,
                date_published,
                source,
                description,
                author,
                author_url,
            }
        })
        .collect())
}

fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let endpoint = "127.0.0.1:8080";

    println!("Starting server at: {:?}", endpoint);
    HttpServer::new(|| {
        App::new()
            .data(Client::default())
            .service(web::resource("/hity").route(web::post().to_async(hity)))
    })
    .bind(endpoint)?
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_items_test() -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        let html = fs::read_to_string("out.html").expect("Unable to read file");
        let actual = get_items(&html).unwrap();

        let expected: Vec<Item> = vec![
            Item {
                count: 2754,
                title: "Przejazd straży pożarnej w trakcie biegu Piotrkowska Rossman".to_string(),
                href: "https://www.wykop.pl/link/4971063/przejazd-strazy-pozarnej-w-trakcie-biegu-piotrkowska-rossman/".to_string(),
                date_published: DateTime::parse_from_rfc3339("2019-05-26T12:26:01+02:00").unwrap().with_timezone(&Local),
                source: "https://m.youtube.com/watch?v=74eMjNg_dmE&feature=youtu.be".to_string(),
                description: "Czy bieganie jest ważniejsze od ratowania życia?".to_string(),
                author: "Bananowy96".to_string(),
                author_url: "https://www.wykop.pl/ludzie/Bananowy96/".to_string(),
            }
        ];

        assert_eq!(actual, expected);

        Ok(())
    }
}
