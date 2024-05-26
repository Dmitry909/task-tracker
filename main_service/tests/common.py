import requests
import json

host = 'http://localhost:4000'


def signup(username: str, password: str):
    json_data = {"username": username, "password": password}
    response = requests.post(f'{host}/signup', json=json_data)
    return response


def login(username: str, password: str):
    json_data = {"username": username, "password": password}
    response = requests.post(f'{host}/login', json=json_data)
    return response


def update_user_data(token: str, json_data: dict):
    response = requests.put(f'{host}/update_user_data', headers={"Authorization": token}, json=json_data)
    return response
