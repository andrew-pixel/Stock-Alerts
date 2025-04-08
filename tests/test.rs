use stockalerts::handlers::{
    function_handler, send_pushbullet_notification, testable_function_handler,
};
use stockalerts::{StockPrice, TestResults};

use lambda_runtime::{Context, LambdaEvent};
use serde_json::{json, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn testPushbullet() {
        let result = send_pushbullet_notification("test", 1, 32.24, 10.0).await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn fulltest() {
        let event = LambdaEvent::new(json!({"event_type": ""}), Context::default());
        let results = function_handler(event).await;

        assert!(results.is_ok());
    }
    #[tokio::test]
    async fn test_close_event() {
        let event = LambdaEvent::new(json!({"event_type": "close"}), Context::default());

        let results = testable_function_handler(event, None).await.unwrap();

        for stock_result in &results.stocks {
            assert_eq!(stock_result.action, "close");
        }
    }
    #[tokio::test]
    async fn test_price_diff() {
        let test_stocks = vec![
            StockPrice {
                name: "TEST1".to_string(),
                lastprice: 101.0, // none
            },
            StockPrice {
                name: "TEST1".to_string(),
                lastprice: 105.0, // 5%+
            },
            StockPrice {
                name: "TEST1".to_string(),
                lastprice: 50.0, // -5%
            },
        ];
        let event = LambdaEvent::new(json!({"event_type": ""}), Context::default());

        let results = testable_function_handler(event, Some(test_stocks))
            .await
            .unwrap();

        println!("{:#?}", results);

        // Example assertions
        for stock_result in &results.stocks {
            if stock_result.percent > 4.0 {
                assert_eq!(stock_result.action, "significant price %");
            } else if stock_result.percent < 4.0 && stock_result.action == "none" {
                assert_eq!(stock_result.action, "none");
            }
        }
    }
}
