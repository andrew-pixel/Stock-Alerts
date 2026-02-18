![image](./notification.png)
Stock Alerts (Python, AWS Lambda, Postgres, Pushbullet, YahooFinance)

A Python AWS Lambda function that tracks stock prices and sends Pushbullet notifications when significant price swings occur. It also alerts based on set crossing prices. 
Alerts and tracked stocks are stored in a supabase database.
    
Setup & Installation
The build process and deployment process is completely done via Github Actions. You can check .github/lambdabuild.yaml to see the build workflow. 


