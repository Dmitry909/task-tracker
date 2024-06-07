import grpc
import tasks_pb2
import tasks_pb2_grpc
import json

host = 'localhost:50051'
channel = grpc.insecure_channel(host)
stub = tasks_pb2_grpc.TaskServiceStub(channel)


def create_task(author_id, text):
    request = tasks_pb2.CreateTaskRequest(author_id=author_id, text=text)
    return stub.CreateTask(request)


def update_task(user_id, task_id, new_text):
    request = tasks_pb2.UpdateTaskRequest(user_id=user_id, task_id=task_id, new_text=new_text)
    return stub.UpdateTask(request)


def delete_task(user_id, task_id):
    request = tasks_pb2.DeleteTaskRequest(user_id=user_id, task_id=task_id)
    return stub.DeleteTask(request)


def get_task(user_id, task_id):
    request = tasks_pb2.GetTaskRequest(user_id=user_id, task_id=task_id)
    return stub.GetTask(request)


def list_tasks(user_id, offset, limit):
    request = tasks_pb2.ListTasksRequest(user_id=user_id, offset=offset, limit=limit)
    return stub.ListTasks(request)
