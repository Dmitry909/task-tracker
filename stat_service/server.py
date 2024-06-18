import os, sys
from concurrent import futures
import grpc
from google.protobuf import empty_pb2

import stat_pb2
import stat_pb2_grpc


class StatService(stat_pb2_grpc.StatServiceServicer):
    def __init__(self):
        # self.conn = psycopg2.connect(host=os.getenv("DATABASE_HOST"),
        #                              port=os.getenv("DATABASE_PORT"),
        #                              dbname=os.getenv("DATABASE_NAME"),
        #                              user=os.getenv("DATABASE_USER"),
        #                              password=os.getenv("DATABASE_PASSWORD"))
        # self.cur = self.conn.cursor()
        pass

    def Healthcheck(self, request, context):
        print(f'request.a: {request.a}')
        return stat_pb2.HealthcheckResponse(request.a ** 2)


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    stat_pb2_grpc.add_StatServiceServicer_to_server(StatService(), server)
    server.add_insecure_port('[::]:50052')
    server.start()
    server.wait_for_termination()


serve()
