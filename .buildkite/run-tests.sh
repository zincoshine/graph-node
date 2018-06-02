#!/bin/bash

# Exit on the first error
set -e

# Print every command we're running
set -x

function set_up {
    # Make Postgres commands available
    export PATH="$PATH:/usr/lib/postgresql/9.6/bin"

    ## Initialize a Postgres database in .postgres
    #initdb -D .postgres -U $(whoami)

    ## Configure Postgres so we can run it as a regular user
    #sed -i -e "s/#unix_socket_directories/unix_socket_directories/g" .postgres/postgresql.conf

    ## Start Postgres
    #pg_ctl -D .postgres start

    # Create a "test" database
    createdb test

    # Export a URL to the test database
    export THEGRAPH_STORE_POSTGRES_DIESEL_URL="postgresql://buildkite@127.0.0.1:5432/test"
}

function tear_down {
    # Delete the test database
    dropdb test

    ## Stop Postgres
    #pg_ctl -D .postgres stop
}

trap tear_down EXIT

# Set everything up
set_up

# Run the test suite
cargo test -- --test-threads=1
