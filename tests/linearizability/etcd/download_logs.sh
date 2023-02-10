#!/bin/bash
for i in {1..102}
do
    if [ $i -lt 10 ];
    then
        log="00${i}";
    elif [ $i -lt 100 ];
    then
        log="0${i}";
    else
        log="${i}";
    fi
    curl "https://raw.githubusercontent.com/ahorn/linearizability-checker/master/jepsen/etcd_${log}.log" > "etcd_${log}.log";
done
