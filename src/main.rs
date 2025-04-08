use dotenvy::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

use lambda_runtime::{service_fn, Context, Error, LambdaEvent};
use yahoo_finance_api as yahoo;

#[derive(Serialize, Debug, Deserialize)]
pub struct StockPrice {
    pub name: String,
    pub lastprice: f64,
}

#[derive(Serialize)]
struct Response {
    req_id: String,
    msg: String,
}

pub async fn function_handler(event: LambdaEvent<Value>) -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let supabase_url = env::var("URL").expect("Missing SUPABASE URL in .env");
    let supabase_key = env::var("APIKEY").expect("Missing SUPABASE KEY in .env");

    let client = Client::new();

    let response = client
        .get(format!("{}/rest/v1/stocks", supabase_url))
        .header("apikey", &supabase_key)
        .header("Authorization", format!("Bearer {}", supabase_key))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let response_text = response.text().await?;
    let stock_prices: Vec<StockPrice> = match serde_json::from_str(&response_text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            Vec::new()
        }
    };

    let provider = yahoo::YahooConnector::new()?;

    let event_type = event
        .payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    for stock in &stock_prices {
        println!("Stock: {}, Price: {}", stock.name, stock.lastprice);
        if let Ok(response) = provider.get_latest_quotes(&stock.name, "1d").await {
            if let Ok(quote) = response.last_quote() {
                let price_diff = ((quote.close - stock.lastprice) / stock.lastprice).abs();
                if price_diff > 0.04 {
                    let positive_negative = if quote.close < stock.lastprice { -1 } else { 1 };
                    updateDatabase(&supabase_url, &supabase_key, quote.close, &stock.name).await?;
                    send_pushbullet_notification(
                        &stock.name,
                        positive_negative,
                        quote.close,
                        price_diff * 100.0,
                    )
                    .await?;
                } else if event_type == "close" {
                    updateDatabase(&supabase_url, &supabase_key, quote.close, &stock.name).await?;
                }
            }
        }
    }

    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    //tracing::init_default_subscriber();

    let func = service_fn(function_handler);
    lambda_runtime::run(func).await?;

    //let event = serde_json::json!({ "test": "data" }); // Simulated event payload
    //let ctx = lambda_runtime::Context::default(); // Dummy context

    // let result = function_handler(event, ctx).await?;
    //println!("Function Result: {}", result);

    Ok(())
}

async fn updateDatabase(
    url: &str,
    key: &str,
    price: f64,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let priceR = (price * 100.0).round() / 100.0;
    let payload = json!({
        "lastprice": priceR
    });

    let update = format!("{}/rest/v1/stocks?name=eq.{}", url, name);
    let response = client
        .patch(&update)
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Updated stock {} with new price: {}", name, price);
    } else {
        println!(
            "Failed to update stock {}: {:?}",
            name,
            response.text().await?
        );
    }
    Ok(())
}
pub async fn send_pushbullet_notification(
    title: &str,
    positive: i32,
    price: f64,
    percent_change: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let access_token = env::var("PUSHAPIKEY").expect("Missing PUSHAPIKEY in .env");

    let title2 = if positive == 1 {
        format!(" {} +{:.2}%", title, percent_change)
    } else {
        format!(" {} -{:.2}%", title, percent_change)
    };
    let body = format!("Price: ${:.2}", price);

    let payload = json!({
        "type": "note",
        "title": title2,
        "body": body
    });

    let PUSHBULLET_API_URL = "https://api.pushbullet.com/v2/pushes";
    let response = client
        .post(PUSHBULLET_API_URL)
        .header("Access-Token", access_token)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Notification sent successfully!");
    } else {
        println!("Failed to send notification: {:?}", response.text().await?);
    }

    Ok(())
}
#[cfg(test)]
pub mod test {
    pub use super::*;
}
pub mod handlers {
    pub use super::function_handler;
    pub use super::send_pushbullet_notification;
    pub use super::testable_function_handler;
}
pub async fn testable_function_handler(
    event: LambdaEvent<Value>,
    test_stocks: Option<Vec<StockPrice>>,
) -> Result<TestResults, Box<dyn std::error::Error>> {
    // separate clone function handler to not break aws lambda

    dotenv().ok();

    let mut stocks_result = if let Some(test_stocks) = test_stocks {
        test_stocks
    } else {
        let supabase_url = env::var("URL").expect("Missing SUPABASE URL in .env");
        let supabase_key = env::var("APIKEY").expect("Missing SUPABASE KEY in .env");

        let client = Client::new();
        let response = client
            .get(format!("{}/rest/v1/stocks", supabase_url))
            .header("apikey", &supabase_key)
            .header("Authorization", format!("Bearer {}", supabase_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let response_text = response.text().await?;
        serde_json::from_str(&response_text)?
    };

    let provider = yahoo::YahooConnector::new()?;

    let event_type = event
        .payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut results = TestResults { stocks: Vec::new() };
    for stock in &stocks_result {
        println!("Stock: {}, Price: {}", stock.name, stock.lastprice);
        if stock.name == "TEST1" {
            let price_diff = ((100.0 - stock.lastprice) / stock.lastprice).abs();
            let mut action = "none";
            if price_diff > 0.04 {
                action = "significant price %";
            } else if event_type == "close" {
                action = "close"
            }
            results.stocks.push(StockR {
                name: stock.name.clone(),
                previousPrice: stock.lastprice,
                currentPrice: 100.0,
                percent: price_diff * 100.0,
                action: action.to_string(),
            });
        } else if let Ok(response) = provider.get_latest_quotes(&stock.name, "1d").await {
            if let Ok(quote) = response.last_quote() {
                let price_diff = ((quote.close - stock.lastprice) / stock.lastprice).abs();
                let mut action = "none";
                if price_diff > 0.04 {
                    let positive_negative = if quote.close < stock.lastprice { -1 } else { 1 };
                    //updateDatabase(&supabase_url, &supabase_key, quote.close, &stock.name).await?;
                    action = "significant price %";
                } else if event_type == "close" {
                    //updateDatabase(&supabase_url, &supabase_key, quote.close, &stock.name).await?;
                    action = "close"
                }
                results.stocks.push(StockR {
                    name: stock.name.clone(),
                    previousPrice: stock.lastprice,
                    currentPrice: quote.close,
                    percent: price_diff * 100.0,
                    action: action.to_string(),
                });
            }
        }
    }

    Ok(results)
}

#[derive(Debug)]
pub struct TestResults {
    pub stocks: Vec<StockR>,
}
#[derive(Debug)]
pub struct StockR {
    pub name: String,
    pub previousPrice: f64,
    pub currentPrice: f64,
    pub percent: f64,
    pub action: String,
}
