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


def create_task(text: str, token: str):
    json_data = {"text": text}
    response = requests.post(f'{host}/create_task', headers={"Authorization": token}, json=json_data)
    return response


def get_task(task_id: int):
    json_data = {"task_id": task_id}
    response = requests.get(f'{host}/get_task', json=json_data)
    return response


def update_task(task_id: int, new_text: str, token: str):
    json_data = {"task_id": task_id, "new_text": new_text}
    response = requests.put(f'{host}/update_task', headers={"Authorization": token}, json=json_data)
    return response


def like(task_id: int, token: str):
    json_data = {"task_id": task_id}
    response = requests.post(f'{host}/like', headers={"Authorization": token}, json=json_data)
    return response


def view(task_id: int, token: str):
    json_data = {"task_id": task_id}
    response = requests.post(f'{host}/view', headers={"Authorization": token}, json=json_data)
    return response


def healthcheck_stat():
    response = requests.get(f'{host}/healthcheck_stat')
    return response

def likes_and_views(task_id: int):
    json_data = {"task_id": task_id}
    response = requests.get(f'{host}/likes_and_views', json=json_data)
    return response

def most_popular_tasks(sort_by_likes: bool):
    json_data = {"sort_by_likes": sort_by_likes}
    response = requests.get(f'{host}/most_popular_tasks', json=json_data)
    return response

def most_popular_users():
    response = requests.get(f'{host}/most_popular_users')
    return response
