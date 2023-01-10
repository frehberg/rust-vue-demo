<template>
  <el-config-provider namespace="ep">
    <div style="display: flex">
      <BaseSide />
      <div style="margin: 0 auto">
        <MessageMonitor msg="Message Monitor" />
      </div>
    </div>
  </el-config-provider>
</template>

<style>
#app {
  text-align: center;
  color: var(--ep-text-color-primary);
}

.element-plus-logo {
  width: 50%;
}
</style>
<script setup lang="ts">
import MessageMonitor from "~/components/MessageMonitor.vue";
</script>