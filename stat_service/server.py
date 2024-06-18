import os, sys
from concurrent import futures
import grpc
from google.protobuf import empty_pb2

import common_pb2
import common_pb2_grpc
import clickhouse_connect

class StatService(common_pb2_grpc.StatServiceServicer):
    def __init__(self):
        self.client = clickhouse_connect.get_client(host='clickhouse')
        print('self.client created', file=sys.stderr)
        result = self.client.command('SELECT 1')
        print(f'SELECT result: {result}', file=sys.stderr)
        self.client.command('''CREATE TABLE likes
            (
                author_id Int64,
                task_id Int64,
                liker_id Int64
            ) ENGINE = ReplacingMergeTree()
            ORDER BY (author_id, task_id, liker_id);''')
        print(f'CREATE TABLE likes finished', file=sys.stderr)
        self.client.command('INSERT INTO likes VALUES (1, 2, 4)')
        print(f'INSERT finished', file=sys.stderr)
        self.client.command('''CREATE TABLE views
            (
                author_id Int64,
                task_id Int64,
                viewer_id Int64
            ) ENGINE = ReplacingMergeTree()
            ORDER BY (author_id, task_id, viewer_id);''')
        print(f'CREATE TABLE views finished', file=sys.stderr)


    def Healthcheck(self, request, context):
        return common_pb2.HealthcheckResponse(aa=request.a ** 2)


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    common_pb2_grpc.add_StatServiceServicer_to_server(StatService(), server)
    server.add_insecure_port('[::]:50052')
    server.start()
    server.wait_for_termination()


serve()
