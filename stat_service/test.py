from test_common import *
import random
from string import ascii_lowercase, digits


def random_str(length):
    return ''.join(random.choice(ascii_lowercase + digits) for _ in range(length))


def do_everything_test():
    healthcheck_response = healthcheck(5)
    assert healthcheck_response.aa == 25

    print('do_everything_test OK')


do_everything_test()
