#!/bin/bash
#
# This script requires 4 arguments: 
# First 3 as port numbers, next are IP address of firewall or server
#
ports="$1 $2 $3"
firewall="$4"

for i in $ports
do
    nmap -Pn --host-timeout 201 --max-retries 0 -p $i $host
    sleep 1
done
