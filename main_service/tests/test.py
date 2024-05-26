from common import *
import random
from string import ascii_uppercase, digits


def random_str(length):
    return ''.join(random.choice(ascii_uppercase + digits) for _ in range(length))


def test_signup_login_update():
    username = random_str(10)
    password = random_str(10)

    signup_resp = signup(username, password)
    assert (signup_resp.status_code == 201)

    login_resp = login(username, password)
    assert(login_resp.status_code == 200)
    token = login_resp.headers["Authorization"]
    assert (len(token) > 10 and len(token) < 1000)

    update_resp = update_user_data(token, {''})

    print('test_signup_login_update OK')


test_signup_login_update()
