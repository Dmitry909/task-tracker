import os
from concurrent import futures
import grpc
import psycopg2
from google.protobuf import empty_pb2

import tasks_pb2
import tasks_pb2_grpc


class TaskService(tasks_pb2_grpc.TaskServiceServicer):
    def init(self):
        print(f'!!!: {os.getenv("DATABASE_HOST")}')
        print(f'!!!: {os.getenv("DATABASE_PORT")}')
        print(f'!!!: {os.getenv("DATABASE_NAME")}')
        print(f'!!!: {os.getenv("DATABASE_USER")}')
        print(f'!!!: {os.getenv("DATABASE_PASSWORD")}')
        exit(101)
        self.conn = psycopg2.connect(host=os.getenv("DATABASE_HOST"),
                                     port=os.getenv("DATABASE_PORT"),
                                     dbname=os.getenv("DATABASE_NAME"),
                                     user=os.getenv("DATABASE_USER"),
                                     password=os.getenv("DATABASE_PASSWORD"))
        self.cur = self.conn.cursor()

    def CreateTask(self, request, context):
        self.cur.execute("INSERT INTO tasks (author_id, text) VALUES (%s, %s) RETURNING task_id;",
                         (request.author_id, request.text))
        task_id = self.cur.fetchone()[0]
        self.conn.commit()
        return tasks_pb2.CreateTaskResponse(task_id=task_id)

    def UpdateTask(self, request, context):
        self.cur.execute(
            "SELECT author_id FROM tasks WHERE task_id = %s;", (request.task_id,))
        task = self.cur.fetchone()
        if not task or task[0] != request.user_id:
            context.abort(grpc.StatusCode.PERMISSION_DENIED,
                          "Permission Denied")

        self.cur.execute("UPDATE tasks SET text = %s WHERE task_id = %s;",
                         (request.new_text, request.task_id))
        self.conn.commit()
        return empty_pb2.Empty()

    def DeleteTask(self, request, context):
        self.cur.execute(
            "SELECT author_id FROM tasks WHERE task_id = %s;", (request.task_id,))
        task = self.cur.fetchone()
        if not task or task[0] != request.user_id:
            context.abort(grpc.StatusCode.PERMISSION_DENIED,
                          "Permission Denied")

        self.cur.execute(
            "DELETE FROM tasks WHERE task_id = %s;", (request.task_id,))
        self.conn.commit()
        return empty_pb2.Empty()

    def GetTask(self, request, context):
        self.cur.execute(
            "SELECT task_id, author_id, text FROM tasks WHERE task_id = %s;", (request.task_id,))
        task = self.cur.fetchone()
        if not task or task[1] != request.user_id:
            context.abort(grpc.StatusCode.PERMISSION_DENIED,
                          "Permission Denied")

        return tasks_pb2.GetTaskResponse(task_id=task[0], author_id=task[1], text=task[2])

    def ListTasks(self, request, context):
        # TODO а здесь порядок гарантирован?
        self.cur.execute("SELECT task_id, author_id, text FROM tasks WHERE author_id = %s LIMIT %s OFFSET %s;",
                         (request.user_id, request.limit, request.offset))

        tasks_rows = self.cur.fetchall()
        tasks_list = [tasks_pb2.Task(
            task_id=row[0], author_id=row[1], text=row[2]) for row in tasks_rows]

        return tasks_pb2.ListTasksResponse(tasks=tasks_list)


def serve():
    print('!!! serve called')
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    tasks_pb2_grpc.add_TaskServiceServicer_to_server(TaskService(), server)
    server.add_insecure_port('[::]:50051')
    server.start()
    server.wait_for_termination()


print('!!! before serve')
serve()