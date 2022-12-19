<script setup lang="ts">
import {ref} from "vue";
import {ElMessage} from 'element-plus'

defineProps<{ msg: string }>();

const count = ref(0);
const activity = ref(10);
const outframe = ref("123#DEADBEEF");
const frames = ref([]);
const service_url = ref("");

const createWs = () => {
  var counter = 0;
  const connection = new WebSocket(
      ((window.location.protocol === "https:") ? "wss://" : "ws://")
      + window.location.host
      + "/ws");

  connection.addEventListener('message', (event) => {
    console.log(event);
    const zeroPadHex = (num, places) => String(num.toString(16)).padStart(places, '0');
    let parsed = JSON.parse(event.data);

    // continues ping from service
    if (parsed.service_url) {
      service_url.value = parsed.service_url;
      activity.value = (activity.value + 4) % 100;
    }

    if (parsed.data) {
      if (frames.value.length > 100) {
        frames.value.shift();
      }
      frames.value.push({id: zeroPadHex(count.value, 8), frame: parsed.data});
      count.value++;
    }

    if (parsed.notice) {
      toast(parsed.notice);
    }
  });

  connection.addEventListener('error', (event) => {
    console.log(event);
    toast_error("lost connection");
  });

  connection.addEventListener('close', (event) => {
    console.log(event);
    toast_error("lost connection");
  });

  return connection;
}

const connection = ref(createWs());

const sendFrame = () => {
  console.log("Sending Frame", outframe)
  connection.value.send(outframe.value);
}

const toast_error = (msg) => {
  ElMessage.error(msg)
}

const toast = (msg) => {
  ElMessage.info(msg)
}

</script>

<template>
  <h1>{{ msg }}</h1>
  <p>
    <el-progress type="circle" :percentage="activity" :color="colors" :width="25"/>
    URL: {{ service_url }}
  </p>
  <el-divider border-style="dashed"/>
  <!-- example components -->
  <el-input v-model="outframe" style="width: 200px; margin: 20px" type="text" placeholder="Id#Data"/>
  <el-button @click="sendFrame">Send Frame</el-button>
  <el-table :data="frames" border style="width: 100%" max-height="600">
    <el-table-column prop="id" label="ID" width="180"/>
    <el-table-column prop="frame" label="Frame"/>
  </el-table>

</template>

<style>
.ep-button {
  margin: 2px;
}
</style>
