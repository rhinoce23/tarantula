set -ex
req=`cat $1`

cd ./proto
ghz --insecure \
  --proto service.proto \
  --call grpc.Service.Tarantula\
  -c 5 -z 1m \
  -d "$req" \
  localhost:8090