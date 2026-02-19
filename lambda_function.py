
import requests
from dotenv import load_dotenv
import os
import yfinance as yf

load_dotenv()
url=os.environ.get("URL")
supaKey=os.environ.get("APIKEY")
pushKey=os.environ.get("PUSHAPIKEY")

def lambda_handler(event, context):
    eventType = event.get("event_type")
    stocks = getStocks()
    alerts = getAlerts()
    processStocks(stocks, alerts, eventType)
        

def getStocks():
    head= {"apikey": supaKey}
    response = requests.get(url + "rest/v1/stocks", headers=head)

    return response.json()

def getAlerts():
    head= {"apikey": supaKey}
    response = requests.get(url + "rest/v1/alerts", headers=head)

    return response.json()

def processStocks( stocks, alerts, eventType):
    requested = {}

    for stock in stocks:

        ticker = yf.Ticker(stock["name"])
        price = ticker.history(period="1d")["Close"].iloc[-1]
        priceDiff = abs((price-stock["lastprice"]) / stock["lastprice"])
        positive = "+"
        if price < stock["lastprice"]:
            positive = "-"
        if priceDiff > 0.04:
            updateDatabase(stock["name"], price)
            sendPushbullet(stock["name"], positive, price, priceDiff * 100)
        elif eventType == "close":
            updateDatabase(stock["name"], price)
    for alr in alerts:
        ticker = yf.Ticker(alr["name"])
        price = ticker.history(period="1d")["Close"].iloc[-1]
        priceDiff = abs((price-alr["lastprice"]) / alr["lastprice"])
        if alr["direction"] == 1: 
            if price > alr["targetprice"]:
                sendAlert(alr["name"], alr["targetprice"], price)
                clearAlert(alr["name"], alr["targetprice"])
        elif price < alr["targetprice"]:
            sendAlert(alr["name"], alr["targetprice"], price)
            clearAlert(alr["name"], alr["targetprice"])

def clearAlert(name, target):
    headers = {
        "apikey": supaKey,
        "Authorization": f"Bearer {supaKey}"
    }
    
    update_url = f"{url}/rest/v1/alerts?name=eq.{name}&targetprice=eq.{target}"
    
    response = requests.delete(update_url, headers=headers)


def updateDatabase(name, price):
    headers = {"apikey": supaKey ,  "Authorization": f"Bearer {supaKey}"}
    update_url = f"{url}/rest/v1/stocks?name=eq.{name}"
    priceR = round(price, 2)
    payload = {"lastprice": priceR}


    res = requests.patch(update_url, headers=headers, json=payload)

def sendAlert(name, targetprice, currentPrice ):
    title = f"{name} Hit target alert price ${targetprice:.2f}"
    body = f"Current price: ${currentPrice:.2f}"
    payload = {
    "type": "note",
    "title": title,
    "body": body
    }
    pushUrl = "https://api.pushbullet.com/v2/pushes"
    headers = {"Access-Token": pushKey}
    res = requests.post(pushUrl, headers=headers, json=payload)
  
def sendPushbullet(name , positive , price, percentChange ):
    title = f"{name} {positive}{percentChange:.2f}"

    body = f"Price: ${price:.2f}"
    payload = {
    "type": "note",
    "title": title,
    "body": body
    }
    pushUrl = "https://api.pushbullet.com/v2/pushes"
    headers = {"Access-Token": pushKey}
    res = requests.post(pushUrl, headers=headers, json=payload)
  
