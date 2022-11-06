#!/bin/bash

cargo build --bin server
npm run build

./target/debug/server
