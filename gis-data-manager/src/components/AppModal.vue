<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  modelValue: Boolean,
  title: String,
  closable: { type: Boolean, default: true },
  closeOnBackdrop: { type: Boolean, default: true },
  wide: Boolean,
})

const emit = defineEmits(['update:modelValue'])
const dialogRef = ref(null)

watch(() => props.modelValue, (val) => {
  if (val) {
    dialogRef.value?.showModal()
  } else {
    dialogRef.value?.close()
  }
})

function onBackdropClick(e) {
  if (!props.closeOnBackdrop) return
  if (e.target === dialogRef.value) {
    emit('update:modelValue', false)
  }
}

function onClose() {
  if (!props.closable) return
  emit('update:modelValue', false)
}
</script>

<template>
  <dialog ref="dialogRef" class="modal" @click="onBackdropClick">
    <div class="modal-box" :class="{ 'max-w-3xl': wide, 'max-w-lg': !wide }">
      <h3 v-if="title" class="font-bold text-lg mb-4">{{ title }}</h3>
      <button v-if="closable" class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
              @click="onClose">✕</button>
      <slot />
      <div v-if="$slots.footer" class="modal-action">
        <slot name="footer" />
      </div>
    </div>
    <form v-if="closable" method="dialog" class="modal-backdrop"><button>close</button></form>
  </dialog>
</template>
