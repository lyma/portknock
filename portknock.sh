#!/bin/bash
#
# This script requires 4 arguments: 
# First 3 as port numbers, next is the IP address of firewall or server
#
# Example: 
# portknock.sh 1273 1422 1271 192.0.2.1
#

ports="$1 $2 $3"
firewall="$4"

for i in $ports
do
    nmap -Pn --host-timeout 201 --max-retries 0 -p $i $firewall
    sleep 1
done
