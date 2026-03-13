
import requests
from dotenv import load_dotenv
import os
import yfinance as yf
#import numpy as np
from datetime import datetime, timezone
load_dotenv()
url=os.environ.get("URL")
supaKey=os.environ.get("APIKEY")
pushKey=os.environ.get("PUSHAPIKEY")
discord=os.environ.get("DISCORD")


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

def checkTightband(last30, last2h):
    shortRange = last30['High'].max() - last30['Low'].min()
    longRange = last2h['High'].max() - last2h['Low'].min()
    tight = shortRange < (longRange * 0.5)
    return tight

def volatility(data, elapsed, currentPrice):
    highEnd = data["High"].max()
    lowEnd = data["Low"].min()
    rangePercent = (highEnd- lowEnd) /  currentPrice

    timeFactor = 1.0-(elapsed/3600)
    
    volatility = timeFactor * rangePercent
    volatility = max(volatility, 0.01)
    #sendDiscord("Current bounds set for "+ str(highEnd*(1+volatility/2)) + " " + str(lowEnd*(1-volatility/2)))
    return volatility

def processStocks( stocks, alerts, eventType):
    names = [s["name"] for s in stocks]
    data = yf.download(names, period="1d",interval="1m", group_by='ticker', threads=True)
    now = datetime.now(timezone.utc)
    timestampSeconds = now.timestamp()

    for stock in stocks:
        #ticker = yf.Ticker(stock["name"])
        #price = ticker.history(period="1d")["Close"].iloc[-1]
        ticker = data.get(stock["name"])
        lastupdateobj = datetime.fromisoformat(stock["lastupdate"])
        lastupdate = lastupdateobj.timestamp()
        elapsed = timestampSeconds - lastupdate
        percentCheck = 0.025
        
        price = ticker["Close"].iloc[-1]
        priceDiff = abs((price-stock["lastprice"]) / stock["lastprice"])
        positive = "+"

        if elapsed > 1800.0 and not stock["band"]:
            updateDatabase(stock["name"], price, now.isoformat(), True)
            percentCheck = volatility(ticker, elapsed, price) / 2 
            upper = price * (1 + percentCheck)
            lower = price * (1 - percentCheck)
            sendDiscord(f"{stock['name']} appears to be trading within a band "f"{lower:.2f}-{upper:.2f}")
            
        elif elapsed > 1800.0:
            percentCheck = volatility(ticker, elapsed, stock["lastprice"]) / 2 
        if price < stock["lastprice"]:
            positive = "-"
        if priceDiff > percentCheck:
            updateDatabase(stock["name"], price, now.isoformat())
            sendPushbullet(stock["name"], positive, price, priceDiff * 100)
        elif eventType == "close":
            updateDatabase(stock["name"], price, now.isoformat())

    for alr in alerts:
        ticker = data.get(alr["name"])
        price = ticker["Close"].iloc[-1]
        priceDiff = abs((price-alr["targetprice"]) / alr["targetprice"])
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


def updateDatabase(name, price, timestamp, band=False):
    headers = {"apikey": supaKey ,  "Authorization": f"Bearer {supaKey}"}
    update_url = f"{url}/rest/v1/stocks?name=eq.{name}"
    priceR = round(price, 2)
    payload = {"lastprice": priceR, "lastupdate": timestamp, "band": band}


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

def sendDiscord(msg):
    data = {
    "content": msg,
    "username": "stock terrapin" # Optional: overrides the default webhook name
    }

    # Send the POST request
    response = requests.post(discord, json=data)

    # Check the response status
    if response.status_code == 204:
        print("Message sent successfully!")
    else:
        print(f"Failed to send message. Status code: {response.status_code}")

def checkSP():
    pass
def option():
    pass
    #ticker.option_chain()
def socialmedia():
    pass
def sentimentnews():
    pass