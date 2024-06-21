from common import *
import random
from string import ascii_lowercase, digits, ascii_uppercase
import json
import time
import clickhouse_connect

def random_str(length):
    return ''.join(random.choice(ascii_lowercase + digits) for _ in range(length))


def random_password(length):
    return ''.join(random.choice(ascii_lowercase + ascii_uppercase + digits) for _ in range(length))


def test_signup_login_update():
    username = random_str(10)
    password = 'aaaaaA1*'

    signup_resp = signup(username, password)
    assert (signup_resp.status_code == 201)

    login_resp = login(username, password)
    assert (login_resp.status_code == 200)
    token = login_resp.headers["Authorization"]
    assert (len(token) > 10 and len(token) < 1000)

    update_resp = update_personal_data(
        token, {'first_name': 'A', 'phone_number': 'B'})
    assert (update_resp.status_code == 200)
    get_resp = get_personal_data(token)
    assert (get_resp.status_code == 200)
    get_dict = json.loads(get_resp.text)
    assert (isinstance(get_dict, dict))
    assert (get_dict['first_name'] == 'A')
    assert (get_dict['second_name'] == None)
    assert (get_dict['email'] == None)
    assert (get_dict['phone_number'] == 'B')

    update_resp = update_personal_data(
        token, {'second_name': 'C', 'phone_number': 'D'})
    get_resp = get_personal_data(token)
    get_dict = json.loads(get_resp.text)
    assert (get_dict['first_name'] == 'A')
    assert (get_dict['second_name'] == 'C')
    assert (get_dict['email'] == None)
    assert (get_dict['phone_number'] == 'D')

    print('test_signup_login_update OK')


def test_tasks():
    username = random_str(10)
    password = 'aaaaaA1*'

    signup(username, password)
    login_resp = login(username, password)
    token = login_resp.headers["Authorization"]

    create_resp1 = create_task('Do please', token)
    assert create_resp1.status_code == 201
    task_id1 = json.loads(create_resp1.text)["task_id"]
    create_resp2 = create_task('Do asap please!!!', token)
    assert create_resp2.status_code == 201
    task_id2 = json.loads(create_resp2.text)["task_id"]

    assert task_id1 + 1 == task_id2

    print('test_tasks OK')


def test_like_view():
    username = random_str(10)
    password = 'aaaaaA1*'

    signup(username, password)
    login_resp = login(username, password)
    token = login_resp.headers["Authorization"]

    create_resp1 = create_task('task text', token)
    assert create_resp1.status_code == 201
    task_id1 = json.loads(create_resp1.text)["task_id"]

    assert like(task_id1, token).status_code == 200
    assert like(task_id1, token).status_code == 200
    assert like(100500, token).status_code == 500 # TODO исправить на 404

    print('test_like_view OK')


def test_stat():
    hc_resp = healthcheck_stat()
    assert hc_resp.status_code == 200
    
    print('test_stat OK')


def test_aggregate():
    client = clickhouse_connect.get_client(host='localhost')
    client.command('TRUNCATE TABLE likes')

    usernames = [random_str(10) for _ in range(5)]
    password = 'aaaaaA1*'

    tokens = []
    for username in usernames:
        signup(username, password)
        tokens.append(login(username, password).headers["Authorization"])

    task_id1_1 = json.loads(create_task('task text', tokens[0]).text)["task_id"]
    task_id2_1 = json.loads(create_task('task text', tokens[1]).text)["task_id"]
    task_id2_2 = json.loads(create_task('task text', tokens[1]).text)["task_id"]

    for token in tokens:
        like(task_id1_1, token)
    for token in tokens:
        like(task_id1_1, token)
    view(task_id1_1, tokens[0])

    for token in tokens[:4]:
        like(task_id2_1, token)
    for token in tokens[:3]:
        like(task_id2_2, token)

    time.sleep(2)

    likes_and_views_resp = likes_and_views(task_id1_1)
    print(likes_and_views_resp.status_code)
    assert likes_and_views_resp.status_code == 200
    likes_and_views_dict = json.loads(likes_and_views_resp.text)
    assert likes_and_views_dict["task_id"] == task_id1_1
    assert likes_and_views_dict["likes_count"] == 5
    assert likes_and_views_dict["views_count"] == 1

    # most_popular_tasks_resp = most_popular_tasks(sort_by_likes=True)
    # assert most_popular_tasks_resp.status_code == 200
    # most_popular_tasks_list = json.loads(most_popular_tasks_resp.text)
    # assert 3 <= len(most_popular_tasks_list) <= 5
    # assert most_popular_tasks_list[0]["task_id"] == task_id1_1
    # assert most_popular_tasks_list[1]["task_id"] == task_id2_1
    # assert most_popular_tasks_list[2]["task_id"] == task_id2_2
    # assert most_popular_tasks_list[0]["likes_count"] == 5
    # assert most_popular_tasks_list[1]["likes_count"] == 4
    # assert most_popular_tasks_list[2]["likes_count"] == 3

    most_popular_users_resp = most_popular_users()
    assert most_popular_users_resp.status_code == 200
    most_popular_users_list = json.loads(most_popular_users_resp.text)
    assert 2 <= len(most_popular_users_list) <= 3
    assert most_popular_users_list[0]["author_username"] == usernames[1]
    assert most_popular_users_list[1]["author_username"] == usernames[0]
    assert most_popular_users_list[0]["likes_count"] == 7
    assert most_popular_users_list[1]["likes_count"] == 5

    print('test_aggregate OK')


# test_signup_login_update()
# test_tasks()
# test_like_view()
# test_stat()
test_aggregate()
