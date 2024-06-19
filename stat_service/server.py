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

class StatService(common_pb2_grpc.StatServiceServicer):
    def __init__(self):
        self.client = clickhouse_connect.get_client(host='clickhouse')
        for query in create_queries:
            self.client.command(query)


    def Healthcheck(self, request, context):
        return common_pb2.HealthcheckResponse(aa=request.a ** 2)


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    common_pb2_grpc.add_StatServiceServicer_to_server(StatService(), server)
    server.add_insecure_port('[::]:50052')
    server.start()
    server.wait_for_termination()


serve()
