import grpc
import common_pb2
import common_pb2_grpc
import json

host = 'localhost:50052'
channel = grpc.insecure_channel(host)
stub = common_pb2_grpc.StatServiceStub(channel)


def healthcheck(a):
    request = common_pb2.HealthcheckRequest(a=a)
    return stub.Healthcheck(request)
