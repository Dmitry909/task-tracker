import grpc
import stat_pb2
import stat_pb2_grpc
import json

host = 'localhost:50052'
channel = grpc.insecure_channel(host)
stub = stat_pb2_grpc.StatServiceStub(channel)


def healthcheck(a):
    request = stat_pb2.HealthcheckRequest(aaa=a)
    return stub.Healthcheck(request)
