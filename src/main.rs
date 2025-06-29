
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

#[derive(Serialize, Debug, Deserialize)]
pub struct Alert {
    pub name: String,
    pub targetprice: f64,
    pub direction: i8
}

#[derive(Serialize)]
struct Response {
    req_id: String,
    msg: String,
}
pub async fn getStocks(client: &Client, url: &str, key:&str) -> Result<Vec<StockPrice>, Box<dyn std::error::Error>> {
    let response = client
        .get(format!("{}/rest/v1/stocks", url))
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let text = response.text().await?;
    Ok(serde_json::from_str(&text)?)
}

pub async fn getAlerts(client: &Client, url: &str, key:&str) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
    let response = client
        .get(format!("{}/rest/v1/alerts", url))
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let text = response.text().await?;
    Ok(serde_json::from_str(&text)?)
}

pub async fn processStocksAndAlerts(stocks : Vec<StockPrice>, alerts: Vec<Alert>, eventType: &str, key : &str, url : &str
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;
    for stock in &stocks {
        println!("Stock: {}, Price: {}", stock.name, stock.lastprice);
        if let Ok(response) = provider.get_latest_quotes(&stock.name, "1d").await {
            if let Ok(quote) = response.last_quote() {
                let price_diff = ((quote.close - stock.lastprice) / stock.lastprice).abs();
                if price_diff > 0.04 {
                    
                    let positive_negative = if quote.close < stock.lastprice { -1 } else { 1 };
                    updateDatabase(url, key, quote.close, &stock.name).await?;
                    send_pushbullet_notification(
                        &stock.name,
                        positive_negative,
                        quote.close,
                        price_diff * 100.0,
                    )
                    .await?;
                } else if eventType == "close" {
                    updateDatabase(url, key, quote.close, &stock.name).await?;
                }
            }
        }
    }
    println!("Total alerts: {}", alerts.len());
    for alr in &alerts {
        if let Ok(response) = provider.get_latest_quotes(&alr.name, "1d").await {
            if let Ok(quote) = response.last_quote(){
                println!(
                        "Latest quote for {} - Close: {}, Target: {}, Direction: {}",
                        alr.name, quote.close, alr.targetprice, alr.direction
                    );
                if alr.direction == 1{
                    if quote.close > alr.targetprice{
                        println!("alert triggered");
                        send_alert(&alr.name, alr.targetprice, quote.close).await?;
                        clear_alert(url, key, &alr.name, alr.targetprice).await?;
                    }
                }
                else{
                    if quote.close < alr.targetprice{
                        send_alert(&alr.name, alr.targetprice, quote.close).await?;
                        clear_alert(url, key, &alr.name, alr.targetprice).await?;
                    }
                }
            }
        }
    }
    Ok(()) 
}
pub async fn function_handler(event: LambdaEvent<Value>) -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let supabase_url = env::var("URL").expect("Missing SUPABASE URL in .env");
    let supabase_key = env::var("APIKEY").expect("Missing SUPABASE KEY in .env");

    let client = Client::new();
    let event_type = event
        .payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let client = Client::new();
    let stocks = getStocks(&client, &supabase_url, &supabase_key).await?;
    let alerts = getAlerts(&client, &supabase_url, &supabase_key).await?;

    processStocksAndAlerts(stocks, alerts, event_type, &supabase_key,&supabase_url).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    //tracing::init_default_subscriber();

    let func = service_fn(function_handler);
    lambda_runtime::run(func).await?;

    //let event = serde_json::json!({ "test": "data" }); // Simulated event payload
    //let ctx = lambda_runtime::Context::default(); // Dummy context

    //let result = function_handler(event).await?;
    //println!("Function Result: {}", result);

    Ok(())
}
async fn clear_alert(
    url: &str,
    key: &str,
    name: &str,
    target: f64
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let target_enc = format!("{}", target);
    let update = format!("{}/rest/v1/alerts?name=eq.{}&targetprice=eq.{}", url, name, target_enc);
    let response = client
        .delete(&update)
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await?;

    if response.status().is_success() {
        println!("Deleted {} ", name);
    } else {
        println!(
            "Failed to delete {}: {:?}",
            name,
            response.text().await?
        );
    }
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
pub async fn send_alert(name: &str, targetprice: f64, quote: f64) -> Result<(), Box<dyn std::error::Error>>{
    let client = Client::new();
    let access_token = env::var("PUSHAPIKEY").expect("Missing PUSHAPIKEY in .env");

    let title = format!(" {} Hit target alert price ${:.2}", name, targetprice);
    
    let body = format!("Current Price: ${:.2}", quote);

    let payload = json!({
        "type": "note",
        "title": title,
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
    test_stocks: Vec<StockPrice>,
    test_alerts: Vec<Alert>,
) -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let supabase_url = env::var("URL")?;
    let supabase_key = env::var("APIKEY")?;

    let event_type = event
        .payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // run pure logic
    processStocksAndAlerts(test_stocks, test_alerts, event_type,  &supabase_key,&supabase_url).await
}
