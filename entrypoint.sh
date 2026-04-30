#!/bin/sh
matchbox_server &
nginx -g 'daemon off;'
