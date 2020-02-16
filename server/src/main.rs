// Based on https://github.com/actix/examples/blob/e8ab9ee7cab3a17aedbddb4800d56d206d0a296f/async_ex1/src/main.rs
use serde::{Deserialize, Serialize};

use std::io;

use actix_web::{
    client::Client,
    error,
    web::{self, BytesMut},
    App, Error as ActixError, HttpResponse, HttpServer,
};

use futures::StreamExt;
use validator::Validate;
use validator_derive::Validate;

use kuchiki::traits::*;
//use kuchiki::NodeRef;
use kuchiki::{ElementData, NodeDataRef};

use derive_more::Display;

use chrono::{DateTime, Local};

use std::fs;
use std::path::Path;

#[derive(Debug, Validate, Deserialize, Serialize)]
struct SomeData {
    #[validate(length(min = 1, max = 1000000))]
    id: String,
    #[validate(length(min = 1, max = 100))]
    name: String,
}

// https://github.com/masnagam/mirakc/blob/4e7bae797439a9b786968c2766155ff214503a49/src/web.rs#L45
#[derive(Serialize)]
struct ErrorBody {
    pub code: u16,
    pub reason: Option<&'static str>,
    pub errors: Vec<u8>,
}

async fn hity(client: web::Data<Client>) -> Result<HttpResponse, ActixError> {
    let mut res = client
        .get("https://www.wykop.pl/hity/dnia/")
        .send()
        .await
        .map_err(ActixError::from)?;

    let mut body = BytesMut::new();
    while let Some(chunk) = res.next().await {
        body.extend_from_slice(&chunk?);
    }

    let body_string = String::from_utf8(body.to_vec()).unwrap();

    let date = Local::now();
    println!("{}", date.format("%Y%m%dT%H%M%S"));
    fs::create_dir_all("output")?;
    fs::write(
        Path::new("output").join(date.format("%Y%m%dT%H%M%S%z.html").to_string()),
        &body_string,
    )?;

    let result = get_items(&body_string);
    match result {
        Ok(items) => {
            if items.is_empty() {
                // https://github.com/KilianKrause/rest-api-with-actix/blob/067970b4757b34a9031b49d730e91e5f60a4412b/src/request_handler.rs#L18
                Err(error::ErrorNotFound(serde_json::json!(ErrorBody {
                    code: actix_web::http::StatusCode::NOT_FOUND.as_u16(),
                    reason: None,
                    errors: Vec::new(),
                })))
            } else {
                Ok(HttpResponse::Ok()
                    .content_type("application/json")
                    .body(serde_json::to_string(&items).expect("serde_json::to_string failed")))
            }
        }
        Err(err) => {
            println!("{:?}", err);
            let msg = format!("Something wrong with get_items: {}", err);
            Err(error::ErrorBadRequest(msg))
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Clone)]
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
    let lis = document
        .select(css_selector)
        .map_err(|()| MyError::MissingItems)?
        .collect::<Vec<NodeDataRef<ElementData>>>();
    println!(">>>>>>>>{}", lis.len());
    //Ok(vec![])
    Ok(lis
        .iter()
        //.take(1)
        .take(40)
        .map(|item| {
            //dbg!(&item);

            //dbg!("{:?}", item.text_contents());

            let item_node: &kuchiki::NodeRef = item.as_node();

            let count_node_data_ref = item_node
                .select_first(".diggbox span:first-of-type")
                .unwrap();
            //println!("count_node_data_ref: {:?}", count_node_data_ref);
            //println!(
            //    "count_node_data_ref.text_contents(): {:?}",
            //    count_node_data_ref.text_contents()
            //);
            let count: u32 = count_node_data_ref.text_contents().parse().unwrap();

            let a_node_data_ref = item_node.select_first("h2 a").unwrap();
            let a_node = a_node_data_ref.as_node();

            let title: String = a_node_data_ref.text_contents();
            //println!("title: {}", title);

            let a_elt = a_node.as_element().unwrap();
            let href = a_elt.attributes.borrow().get("href").unwrap().to_string();
            //println!("href: {:?}", href);

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
            //println!("date_published: {:?}", date_published);

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

            //println!("source: {:?}", source);

            let description = item_node
                .select_first(".description a")
                .unwrap()
                .as_node()
                .text_contents()
                .trim()
                .to_string();

            //println!("description: {:?}", description);

            let author_node_data_ref = item_node.select_first("a[href*='ludzie'").unwrap();

            let author_node = author_node_data_ref.as_node();
            //println!("author_node: {:?}", author_node);

            //let author = author_node
            //    .text_contents()
            //    .trim()
            //    // TODO: Get only text node
            //    // https://stackoverflow.com/questions/56329121/how-to-get-only-text-node-with-kuchiki
            //    // https://users.rust-lang.org/t/how-to-get-only-text-node-with-kuchiki/29084
            //    .trim_start_matches('@')
            //    .to_string();

            let author = match author_node_data_ref
                .as_node()
                .children()
                .text_nodes()
                .last()
            {
                Some(x) => x.borrow().clone().trim().to_string(),
                None => String::from(""),
            };

            //println!("author: {:?}", author);

            let author_url = author_node
                .as_element()
                .unwrap()
                .attributes
                .borrow()
                .get("href")
                .unwrap()
                .to_string();

            //println!("author_url: {:?}", author_url);

            //let author_nodes: Vec<NodeRef> = author_node
            //    .descendants()
            //    .filter(|node| {
            //        println!("node: {:?}", node);
            //        true
            //    })
            //    .collect();

            //println!("author_nodes: {:?}", author_nodes);

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

#[actix_rt::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let endpoint = "127.0.0.1:8088";

    println!("Starting server at: {:?}", endpoint);
    HttpServer::new(|| {
        App::new()
            .data(Client::default())
            .service(web::resource("/hity").route(web::get().to(hity)))
    })
    .bind(endpoint)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_items_test() -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        let html = fs::read_to_string("out.html").expect("Unable to read file");
        let items = get_items(&html).unwrap();
        let actual = vec![items[0].clone()];

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

        println!(">>>>>>>>{}", items.len());
        println!(">>>>>>>>{:?}", items[1]);

        Ok(())
    }
}
