import os, sys
from concurrent import futures
import grpc
import psycopg2
from google.protobuf import empty_pb2
from kafka import KafkaProducer
import json

import common_pb2
import common_pb2_grpc

class TaskService(common_pb2_grpc.TaskServiceServicer):
    def __init__(self):
        print('__init__ called', file=sys.stderr)
        self.conn = psycopg2.connect(host=os.getenv("DATABASE_HOST"),
                                     port=os.getenv("DATABASE_PORT"),
                                     dbname=os.getenv("DATABASE_NAME"),
                                     user=os.getenv("DATABASE_USER"),
                                     password=os.getenv("DATABASE_PASSWORD"))
        self.cur = self.conn.cursor()

        self.producer = KafkaProducer(
            bootstrap_servers=['kafka:29092'],
            api_version=(0, 11, 5),
            value_serializer=lambda v: json.dumps(v).encode('utf-8'),
            request_timeout_ms=3000
        )

    def get_author_id_of_task(self, task_id):
        print('get_author_id_of_task called', file=sys.stderr)
        self.cur.execute("SELECT author_id FROM tasks WHERE task_id = %s;", (task_id,))
        print('execute called', file=sys.stderr)
        row = self.cur.fetchone()
        print(f'type(row): {type(row)}', file=sys.stderr)
        if not row:
            return None
        return row

    def CreateTask(self, request, context):
        if not request.author_id or not request.text:
            raise ValueError("author_id or text is missing or empty")
        self.cur.execute("INSERT INTO tasks (author_id, text) VALUES (%s, %s) RETURNING task_id;",
                         (request.author_id, request.text))
        task_id = self.cur.fetchone()[0]
        self.conn.commit()
        return common_pb2.CreateTaskResponse(task_id=task_id)

    def UpdateTask(self, request, context):
        if not request.user_id or not request.task_id or not request.new_text:
            raise ValueError("user_id, task_id or new_text is missing or empty")
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
        if not request.user_id or not request.task_id:
            raise ValueError("user_id or task_id is missing or empty")
        self.cur.execute("SELECT author_id FROM tasks WHERE task_id = %s;", (request.task_id,))
        task = self.cur.fetchone()
        if not task or task[0] != request.user_id:
            context.abort(grpc.StatusCode.PERMISSION_DENIED,
                          "Permission Denied")

        self.cur.execute(
            "DELETE FROM tasks WHERE task_id = %s;", (request.task_id,))
        self.conn.commit()
        return empty_pb2.Empty()

    def GetTask(self, request, context):
        if not request.task_id:
            raise ValueError("user_id or task_id is missing or empty")
        self.cur.execute("SELECT task_id, author_id, text FROM tasks WHERE task_id = %s;", (request.task_id,))
        task = self.cur.fetchone()
        if not task:
            context.abort(grpc.StatusCode.NOT_FOUND, "Task doesn't exist")

        return common_pb2.GetTaskResponse(task_id=task[0], author_id=task[1], text=task[2])

    def ListTasks(self, request, context):
        # if not request.user_id or not request.offset or not request.limit:
        #     raise ValueError("user_id, offset or limit is missing or empty")
        # TODO а здесь порядок гарантирован?
        self.cur.execute("SELECT task_id, author_id, text FROM tasks WHERE author_id = %s LIMIT %s OFFSET %s;",
                         (request.user_id, request.limit, request.offset))

        tasks_rows = self.cur.fetchall()
        tasks_list = [common_pb2.Task(task_id=row[0], author_id=row[1], text=row[2]) for row in tasks_rows]

        return common_pb2.ListTasksResponse(tasks=tasks_list)

    def SendLike(self, request, context):
        print('SendLike called', file=sys.stderr)
        author_id = self.get_author_id_of_task(request.task_id)
        print(f'author_id: {author_id}', file=sys.stderr)
        print(f'type(author_id): {type(author_id)}, author_id: {author_id}', file=sys.stderr)
        if not author_id:
            raise ValueError("No such task_id")
        send_result = self.producer.send('queue_likes', {
            'task_id': request.task_id,
            'author_id': author_id,
            'liker_id': request.liker_id,
        })
        print(f'send_result: {send_result}', file=sys.stderr)
        return common_pb2.EmptyMessage()

    def SendView(self, request, context):
        author_id = self.get_author_id_of_task(request.task_id)
        if not author_id:
            raise ValueError("No such task_id")
        send_result = self.producer.send('queue_views', {
            'task_id': request.task_id,
            'author_id': author_id,
            'liker_id': request.liker_id,
        })
        print(f'send_result: {send_result}', file=sys.stderr)
        return common_pb2.EmptyMessage()


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    common_pb2_grpc.add_TaskServiceServicer_to_server(TaskService(), server)
    server.add_insecure_port('[::]:50051')
    server.start()
    server.wait_for_termination()


serve()
