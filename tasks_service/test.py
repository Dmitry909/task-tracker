from test_common import *
import random
from string import ascii_lowercase, digits


def random_str(length):
    return ''.join(random.choice(ascii_lowercase + digits) for _ in range(length))


def do_everything_test():
    author_id = random.randint(100, 999)
    text1 = random_str(10)
    
    task_id1 = create_task(author_id, text1).task_id
    task1 = get_task(task_id1)
    assert task1.task_id == task_id1 and task1.author_id == author_id and task1.text == text1

    text2 = random_str(10)
    update_task(author_id, task_id1, text2)
    task2 = get_task(task_id1)
    assert task2.task_id == task_id1 and task2.author_id == author_id and task2.text == text2

    text3 = random_str(10)
    task_id3 = create_task(author_id, text3).task_id
    text4 = random_str(10)
    task_id4 = create_task(author_id, text4).task_id
    text5 = random_str(10)
    task_id5 = create_task(author_id, text5).task_id

    list_response = list_tasks(author_id, 0, 100)
    ids = [task.task_id for task in list_response.tasks]
    assert ids == [task_id1, task_id3, task_id4, task_id5]

    delete_task(author_id, task_id3)
    task_id6 = create_task(author_id, text5).task_id
    list_response = list_tasks(author_id, 1, 2)
    ids = [task.task_id for task in list_response.tasks]
    assert ids == [task_id4, task_id5]

    delete_task(author_id, task_id1)
    delete_task(author_id, task_id4)
    delete_task(author_id, task_id5)
    delete_task(author_id, task_id6)

    print('do_everything_test OK')


do_everything_test()
