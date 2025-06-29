use stockalerts::handlers::{
    function_handler, send_pushbullet_notification, testable_function_handler,
};
use reqwest::Client;
use stockalerts::{StockPrice, Alert, getStocks};

use lambda_runtime::{Context, LambdaEvent};
use serde_json::{json, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn testPushbullet() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        send_pushbullet_notification("test", 1, 32.24, 10.0).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testHandlerNothing() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let event = LambdaEvent::new(json!({ "event_type": "" }), Context::default());
        function_handler(event).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testHandlerClose() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let event = LambdaEvent::new(json!({ "event_type": "close" }), Context::default());

        let test_stocks = vec![
            StockPrice {
                name: "AAPL".to_string(),
                lastprice: 100.0,
            },
            StockPrice {
                name: "AAPL".to_string(),
                lastprice: 200.0,
            },
        ];

        let alerts = vec![];
        testable_function_handler(event, test_stocks, alerts).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testPriceDiffAlerts() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let test_stocks = vec![
            StockPrice {
                name: "AAPL".to_string(),
                lastprice: 140.0,
            },
            StockPrice {
                name: "AAPL".to_string(),
                lastprice: 150.0,
            },
            StockPrice {
                name: "AAPL".to_string(),
                lastprice: 135.0,
            },
        ];

        let alerts = vec![
            Alert {
                name: "AAPL".to_string(),
                targetprice: 145.0,
                direction: 1,
            },
            Alert {
                name: "AAPL".to_string(),
                targetprice: 130.0,
                direction: 0,
            },
        ];

        let event = LambdaEvent::new(json!({ "event_type": "" }), Context::default());
        testable_function_handler(event, test_stocks, alerts).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testAlertUpward() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let test_stocks = vec![StockPrice {
            name: "AAPL".to_string(),
            lastprice: 150.0,
        }];

        let alerts = vec![Alert {
            name: "AAPL".to_string(),
            targetprice: 145.0,
            direction: 1,
        }];

        let event = LambdaEvent::new(json!({ "event_type": "" }), Context::default());
        testable_function_handler(event, test_stocks, alerts).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testAlertDownward() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let test_stocks = vec![StockPrice {
            name: "AAPL".to_string(),
            lastprice: 125.0,
        }];

        let alerts = vec![Alert {
            name: "AAPL".to_string(),
            targetprice: 130.0,
            direction: 0,
        }];

        let event = LambdaEvent::new(json!({ "event_type": "" }), Context::default());
        testable_function_handler(event, test_stocks, alerts).await?;
        Ok(())
    }

    #[tokio::test]
    async fn testNoAlert() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        let test_stocks = vec![StockPrice {
            name: "GOOG".to_string(),
            lastprice: 1200.0,
        }];

        let alerts = vec![Alert {
            name: "AAPL".to_string(),
            targetprice: 130.0,
            direction: 1,
        }];

        let event = LambdaEvent::new(json!({ "event_type": "" }), Context::default());
        testable_function_handler(event, test_stocks, alerts).await?;
        Ok(())
    }
}
