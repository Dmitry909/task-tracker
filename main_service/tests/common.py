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


def update_personal_data(token: str, json_data: dict):
    response = requests.put(f'{host}/personal_data', headers={"Authorization": token}, json=json_data)
    return response


def get_personal_data(token: str):
    response = requests.get(f'{host}/personal_data', headers={"Authorization": token})
    return response


def healthcheck_stat():
    response = requests.get(f'{host}/healthcheck_stat')
    return response
