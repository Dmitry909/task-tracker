from common import *
import random
from string import ascii_lowercase, digits, ascii_uppercase


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


test_signup_login_update()
