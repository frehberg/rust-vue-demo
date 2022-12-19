# rust-vue-demo

This project demonstrates the integration of npm/vue into a rust axum web-service. The web-service is self-contained, embedding the webui assets into the binary. The webui creates a websocket being used by the web-service to send data to the webui once per second.

The Vue web framework does not require any template rendering during runtime in the web-service, in contrast to web frameworks such as bootstrap, etc. Vue is based on static HTML/CSS/JS files, being delivered to web-browser as is. Costly DOM tree manipulation is handled by the web-browser.

Avoiding template engines in the web-service (template engines are performing runtime code generation), instead the Vue application and components represent a fixed code snapshot whose state transitions can be tested in the release-process. 

## Requirements
* npm/nodes toolchain must be available
* rust toolchain must be available

## Features
* Vue project code is located in folder `webui/`
* The `cargo build` will trigger the vue npm build process ('npm_rs'), the resulting HTML code will be placed in `webui/dist/`
* The vue JavaScript assets of `webui/dist/` will be embedded into the Rust code (`rust_embed`)
* No costly template rendering of web framework within web-service; all asset files are sent to web-brwoser as is. Costly template subsitution is performed during npm compile time and DOM tree manipulation is performed by web-browser.
* No runtime code generation (no template engines), instead using Vue components, achieving testability of frontend code. 
* The compact executable will be created from Rust code
* The binary will provide a web-service listening at port 3000 (`axum`)
* When connecting with web-browser to service port, eg http://127.0.0.1:3000, a websocket will be established
* The web-service will use the websocket to send data to the webui, cycling once per second.
* The webui provides a button to send data to the webservice.
* If files are modified in `src/` or `webui/src/` the command `cargo build` will update the binary (`build.rs`)


## Usage

Download the project folder from https://github.com/frehberg/rust-vue-demo
```
git clone https://github.com/frehberg/rust-vue-demo
```

Initialize the npm dependencies of the webui frontend
```
cd rust-vue-demo/webui; npm install
```

Build the project
```shell
cd rust-vue-demo/
cargo build
```

Set up the vcan0 device
```shell
sudo modprobe vcan
sudo ip link add vcan0 type vcan
sudo ip link set vcan0 up
```

Start the web-service
```shell
cd rust-vue-demo/
CANDEV="vcan0" cargo run
```

Start the CAN bus dump tool
```shell
candump vcan0
```
Connect with web-browser to http://127.0.0.1:3000

The Web-page will open in browser and will establish a websocket connection to ws://127.0.0.1:3000/ws. This websocket is used to send data updates between webui and web-service.


## Developing the Vue Web Frontend

Generate the assets from Vue templates
```shell
cd rust-vue-demo/webui
npm install; npm run build
```
Start the web-service locally listening at port 8080
```shell
cd rust-vue-demo/webui
npm run serve
```

Connect with web-browser to `http://localhost:8080`

Any time the files in folder `rust-vue-demo/webui/src/` are modified, the npm-build-process will be triggered and the browser will perform a reload.



