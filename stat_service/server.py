import os, sys
from concurrent import futures
import grpc
from google.protobuf import empty_pb2

import common_pb2
import common_pb2_grpc
import clickhouse_connect

create_queries = ['''
CREATE TABLE IF NOT EXISTS kafka_likes (
    author_id Int64,
    task_id Int64,
    liker_id Int64
) ENGINE = Kafka
SETTINGS
    kafka_broker_list = 'kafka:29092',
    kafka_topic_list = 'queue_likes',
    kafka_group_name = 'queue_likes_group',
    kafka_format = 'JSONEachRow',
    kafka_num_consumers = 1;
''',
'''
CREATE TABLE IF NOT EXISTS likes (
    author_id Int64,
    task_id Int64,
    liker_id Int64
) ENGINE = MergeTree()
ORDER BY task_id;
''',
'''
CREATE MATERIALIZED VIEW IF NOT EXISTS kafka_to_likes_mv TO likes AS
SELECT author_id, task_id, liker_id FROM kafka_likes;
''',
'''
CREATE TABLE IF NOT EXISTS kafka_views (
    author_id Int64,
    task_id Int64,
    viewer_id Int64
) ENGINE = Kafka
SETTINGS
    kafka_broker_list = 'kafka:29092',
    kafka_topic_list = 'queue_views',
    kafka_group_name = 'queue_views_group',
    kafka_format = 'JSONEachRow',
    kafka_num_consumers = 1;
''',
'''
CREATE TABLE IF NOT EXISTS views (
    author_id Int64,
    task_id Int64,
    viewer_id Int64
) ENGINE = MergeTree()
ORDER BY task_id;
''',
'''
CREATE MATERIALIZED VIEW IF NOT EXISTS kafka_to_views_mv TO views AS
SELECT author_id, task_id, viewer_id FROM kafka_views;
''']

def parse(li):
    if not isinstance(li, list):
        return []
    res = []
    curr = []
    for q in li:
        if '\n' in q:
            curr.append(q.split('\n')[0])
            res.append(curr.copy())
            curr = [q.split('\n')[1]]
        else:
            curr.append(q)
    res.append(curr.copy())
    return res


class StatService(common_pb2_grpc.StatServiceServicer):
    def __init__(self):
        self.client = clickhouse_connect.get_client(host='clickhouse')
        for query in create_queries:
            self.client.command(query)


    def Healthcheck(self, request, context):
        return common_pb2.HealthcheckResponse(aa=request.a ** 2)

    def GetLikesAndViews(self, request, context):
        query_likes = f'SELECT COUNT(DISTINCT liker_id) FROM likes WHERE task_id == {request.task_id};'
        query_views = f'SELECT COUNT(DISTINCT viewer_id) FROM views WHERE task_id == {request.task_id};'

        print(f"self.client.command('SELECT 1;'): {self.client.command('SELECT 1;')}", file=sys.stderr)
        print(f"self.client.command('SELECT * FROM likes;'): {self.client.command('SELECT * FROM likes;')}", file=sys.stderr)

        print(query_likes, file=sys.stderr)
        resp_likes = self.client.command(query_likes)
        resp_views = self.client.command(query_views)
        print(resp_likes, file=sys.stderr)
        print(resp_views, file=sys.stderr)
        return common_pb2.GetLikesAndViewsResponse(task_id=request.task_id, likes_count=resp_likes, views_count=resp_views)

    def GetTop5Posts(self, request, context):
        query = ''
        if request.sort_by_likes:
            query = 'SELECT task_id, COUNT(DISTINCT liker_id) AS unique_likes FROM likes GROUP BY task_id ORDER BY unique_likes DESC LIMIT 5;'
        else:
            query = 'SELECT task_id, COUNT(DISTINCT viewer_id) AS unique_views FROM views GROUP BY task_id ORDER BY unique_views DESC LIMIT 5;'

        resp = parse(self.client.command(query))
        print(resp, file=sys.stderr)
        if request.sort_by_likes:
            return common_pb2.GetTop5PostsResponse(posts=[
                common_pb2.GetTop5PostsResponseOne(task_id=int(row[0]), author_id=0, likes_count=int(row[1]), views_count=0) for row in resp
            ])

        return common_pb2.GetTop5PostsResponse(posts=[
            common_pb2.GetTop5PostsResponseOne(task_id=int(row[0]), author_id=0, likes_count=0, views_count=int(row[0])) for row in resp
        ])

    def GetTop3Users(self, request, context):
        query = '''
                SELECT 
                    author_id,
                    COUNT(DISTINCT task_id, liker_id) AS unique_likes
                FROM likes
                GROUP BY author_id
                ORDER BY unique_likes DESC
                LIMIT 3;
                '''
        resp = self.client.command(query)
        resp = parse(resp)
        print(resp, file=sys.stderr)
        return common_pb2.GetTop3UsersResponse(users=[
            common_pb2.GetTop3UsersResponseOne(author_id=int(row[0]), likes_count=int(row[1])) for row in resp
        ])


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    common_pb2_grpc.add_StatServiceServicer_to_server(StatService(), server)
    server.add_insecure_port('[::]:50052')
    server.start()
    server.wait_for_termination()


serve()
