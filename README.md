 Stock Alerts - Work in Progress

A Rust-based AWS Lambda function that tracks stock prices and sends Pushbullet notifications when significant price swings occur. It also updates a Supabase database with the latest stock prices.

 This project is a work in progress. Future plans include:
 Adding a user interface
 Custom alerts for specific price targets
 More refined alert conditions

 How It Works
Runs every 10 minutes via AWS EventBridge.
Fetches stock data from Supabase and updates prices from Yahoo Finance API.
If a stock price changes by more than 4%, it:
    Sends a Pushbullet notification.
    Updates the Supabase database.
At market close (5PM), it forces an update even if no major price swings happened.

Tech Stack

Rust
AWS Lambda (for execution)
EventBridge (to trigger function)
Yahoo Finance API (for stock data)
Supabase (for database)
Pushbullet (for notifications)
    
Setup & Installation
The build process and deployment process is completely done via Github Actions. You can check .github/lambdabuild.yaml to see the build workflow. 

I built the project using cargo lambda build --release in MSYS2. The build process was quite problematic with a lot of dependency errors. In the future i plan on trying docker for building the project.
After building just upload the build to Lambda and setup AWS Eventbridge to trigger your alerts. 
You will also need a .env file to setup the keys for pushbullet and supabase.
