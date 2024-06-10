#!/usr/bin/env bash

docker stop running_telebot
docker rm running_telebot
docker run --name running_telebot --env-file .env -d jmarcelomb/telebot:latest
