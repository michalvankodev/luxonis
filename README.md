# Luxonis test task

## Installation

1. Ensure you have Rust development environment installed. Using [rustup](https://rustup.rs/) is recommended to get up and running quickly.
   Follow instructions for setting up rustup toolchain.

2. clone the repository 

`git clone git@github.com:michalvankodev/luxonis.git` or `git clone https://github.com/michalvankodev/luxonis.git`

This should be enough. All application dependencies will be downloaded when the project is being compiled.
For compilation I recommend using [`cargo`](https://doc.rust-lang.org/cargo/)

## Running application server

`cargo run --bin server`

> Server will be running on port 3301. Make sure it is available.

## Running application clients

Clients can connect to server through TCP connection or UNIX socket.

### Connection through TCP client

As the server is running on port 3301. Locate designed IP address of the server.
In this example we are going to connect on `localhost`

`cargo run --bin client 127.0.0.1:3301`

### Connection through UNIX socket

Socket is created in `/tmp` folder: 

`cargo run --bin client /tmp/luxonis.sock`


## Gameplay

Game is played in CLI interface

Navigating menu is done by typing a **number** presented on screen.

### Example of main menu

User is presented with following menu:

   Please specify what action you would like to take by typing a number:

   (0) Quit
   (1) List and challenge available opponents

To proceed it has to type either `0` or `1` to continue.

All users that are not in game are available for a challenge.
