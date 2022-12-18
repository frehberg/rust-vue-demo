<template>

  <div id="app" class="h-screen">
    <h1 class="mb-4 text-4xl font-extrabold tracking-tight leading-none text-gray-900 md:text-5xl lg:text-6xl dark:text-white">Data Monitor</h1>
    <div class="my-2">
       URL: {{ service_url }}
    </div>
    <p class="error" v-if="notice">{{ notice }}</p>
    <div class="h-0.5 bg-gray-200 w-36 mx-auto mt-2.5"></div>

    <form>
      <div class="flex justify-center items-center py-2">
        <input class="appearance-none bg-transparent border-none text-gray-700 mr-3 py-1 px-2 leading-tight focus:outline-none" type="text" placeholder="123#DEADBEEF" aria-label="frame">
        <button class="flex-shrink-0 bg-teal-500 hover:bg-teal-700 border-teal-500 hover:border-teal-700 text-sm border-4 text-white py-1 px-2 rounded" type="button">
          Send Frame
        </button>
      </div>
    </form>

    <div class="border">
      <ul>
        <li v-for="message in messages" v-bind:key="message">
          {{ message }}
        </li>
      </ul>

    </div>
  </div>
</template>

<script>

export default {
  name: 'App',
  data: function () {
    return {
      messages: [],
      connection: null,
      counter: 0,
      service_url: "unknown",
      frame: "123#DEADBEEF",
    }
  },
  methods: {
    sendMessage: function (frame) {
      console.log("Send CAN frame", frame)
      this.connection.send(frame);
      const zeroPadHex = (num, places) => String(num.toString(16)).padStart(places, '0');

      if (this.messages.length > 20) {
        this.messages.shift();
      }
      this.messages.push("<--  " + zeroPadHex(this.counter, 8) + ": " + frame);
    }
  },


  created: function () {
    let vm = this
    console.log("Starting connection to WebSocket Server")
    this.connection = new WebSocket(((window.location.protocol === "https:") ? "wss://" : "ws://") + window.location.host + "/ws");
    this.connection.onmessage = function (event) {
      const zeroPadHex = (num, places) => String(num.toString(16)).padStart(places, '0');

      console.log(event);
      let parsed = JSON.parse(event.data);
      vm.counter += 1;
      vm.service_url = parsed.service_url;
      if (parsed.data) {
        if (vm.messages.length > 15) {
          vm.messages.shift();
        }
        vm.messages.push("------>  "
            + zeroPadHex(vm.counter, 8)
            + ": " + parsed.data)
      }

      if (parsed.notice) {
        vm.notice = parsed.notice
      }
    }

    this.connection.onopen = function (event) {
      console.log(event)
      console.log("Successfully connected to the echo websocket server...")
    }

  }
}
</script>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
  margin-top: 60px;
}

.error {
  color: red;
}

.message {
  float: left
}
</style>


