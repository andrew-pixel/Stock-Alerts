
import requests
import logging
import boto3
from dotenv import load_dotenv
import os
import yfinance as yf
# Initialize the S3 client outside of the handler
s3_client = boto3.client('s3')

# Initialize the logger
logger = logging.getLogger()
logger.setLevel("INFO")
load_dotenv()
url=os.environ.get("SUPABASE_URL")
supaKey=os.environ.get("SUPABASEKEY")
pushKey=os.environ.get("PUSH_KEY")

def upload_receipt_to_s3(bucket_name, key, receipt_content):
    """Helper function to upload receipt to S3"""
    
    try:
        s3_client.put_object(
            Bucket=bucket_name,
            Key=key,
            Body=receipt_content
        )
    except Exception as e:
        logger.error(f"Failed to upload receipt to S3: {str(e)}")
        raise

def lambda_handler(event, context):
    """
    Main Lambda handler function
    Parameters:
        event: Dict containing the Lambda function event data
        context: Lambda runtime context
    Returns:
        Dict containing status message
    """
    try:
        # Parse the input event
        order_id = event['Order_id']
        amount = event['Amount']
        item = event['Item']
        
        # Access environment variables
        bucket_name = os.environ.get('RECEIPT_BUCKET')
        if not bucket_name:
            raise ValueError("Missing required environment variable RECEIPT_BUCKET")

        # Create the receipt content and key destination
        receipt_content = (
            f"OrderID: {order_id}\n"
            f"Amount: ${amount}\n"
            f"Item: {item}"
        )
        key = f"receipts/{order_id}.txt"

        # Upload the receipt to S3
        upload_receipt_to_s3(bucket_name, key, receipt_content)

        logger.info(f"Successfully processed order {order_id} and stored receipt in S3 bucket {bucket_name}")
        
        return {
            "statusCode": 200,
            "message": "Receipt processed successfully"
        }

    except Exception as e:
        logger.error(f"Error processing order: {str(e)}")
        raise


def getStocks():
    head= {"apikey": supaKey}
    response = requests.get(url + "rest/v1/stocks", headers=head)

    return response.json()

def getAlerts():
    head= {"apikey": supaKey}
    response = requests.get(url + "rest/v1/alerts", headers=head)

    return response.json()

def processStocks( stocks, alerts, eventType):
    for stock in stocks:
        print(stock.name, stock.lastprice)
        ticker = yf.Ticker(stock.name)
        price = ticker.history(period="1d")["Close"].iloc[-1]
        print(price)

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

