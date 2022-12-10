<template>
  <div id="app">
    <h2>Data Monitor</h2>
    {{ service_url }}
    <br />
    Message Counter: {{ counter }}
    <br />
    <button v-on:click="sendMessage('hello')">Send Hello</button>
    <br />
    <p style="white-space: pre-line">
    <br />
    </p>
    <div>
     <dl class="list-group list-group-flush text-left">
        <dl class="list-group-item" v-for="message in messages" v-bind:key="message" >
           <span >
              {{message}}
           </span>
        </dl>
     </dl>
    </div>
  </div>
</template>

<script>

export default {
  name: 'App',
  data: function() {
    return {
      messages: [],
      connection: null,
      counter: 0,
      service_url: "unknown",
    }
  },
  methods: {
    sendMessage: function(message) {
      console.log("Hello")
      console.log(this.connection);
      this.connection.send(message);
    }
  },

  created: function() {
    let vm = this
    console.log("Starting connection to WebSocket Server")
    this.connection = new WebSocket(((window.location.protocol === "https:") ? "wss://" : "ws://") + window.location.host + "/ws");
    this.connection.onmessage = function(event) {
      const zeroPadHex = (num, places) => String(num.toString(16)).padStart(places, '0');
      
      console.log(event);
      if (vm.messages.length > 20) {
        vm.messages.shift(); 
      }
      let parsedMessage = JSON.parse(event.data);
      vm.counter += 1;
      vm.service_url = parsedMessage.service_url;
      vm.messages.push(zeroPadHex(parsedMessage.counter, 8) + ": " + parsedMessage.body)
    }

    this.connection.onopen = function(event) {
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
</style>


