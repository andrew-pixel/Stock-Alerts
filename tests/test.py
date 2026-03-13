import requests

import os
url=os.environ.get("URL")
supaKey=os.environ.get("APIKEY")
pushKey=os.environ.get("PUSHAPIKEY")
discord=os.environ.get("DISCORD")


import yfinance as yf
import time



def test_getStocks(mock_get):
    mock_get.return_value.json.return_value = [{"name": "AAPL", "lastprice": 100}]
    stocks = getStocks()
    assert stocks[0]["name"] == "AAPL"
    mock_get.assert_called_once()


def test_getAlerts(mock_get):
    mock_get.return_value.json.return_value = [{"name": "AAPL", "targetprice": 150, "direction": 1}]
    alerts = getAlerts()
    assert alerts[0]["targetprice"] == 150
    mock_get.assert_called_once()



def test_updateDatabase(mock_patch):
    updateDatabase("AAPL", 123.456)
    mock_patch.assert_called_once()
    args, kwargs = mock_patch.call_args
    assert "AAPL" in args[0]
    assert kwargs["json"]["lastprice"] == round(123.456, 2)


def test_clearAlert(mock_delete):
    clearAlert("AAPL", 150.0)
    mock_delete.assert_called_once()
    args, kwargs = mock_delete.call_args
    assert "AAPL" in args[0]
    assert "150.0" in args[0]


def test_sendPushbullet(mock_post):
    sendPushbullet("AAPL", "+", 150.0, 5.5)
    mock_post.assert_called_once()
    args, kwargs = mock_post.call_args
    payload = kwargs["json"]
    assert payload["type"] == "note"
    assert "AAPL" in payload["title"]


def test_sendAlert(mock_post):
    sendAlert("AAPL", 150.0, 155.0)
    mock_post.assert_called_once()
    payload = mock_post.call_args.kwargs["json"]
    assert "Current price" in payload["body"]



