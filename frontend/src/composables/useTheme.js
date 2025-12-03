import { ref, watch, onMounted } from 'vue';

export function useTheme() {
  // 默认使用亮色主题，如果localStorage中没有设置则默认为false
  const storedTheme = localStorage.getItem('darkMode');
  const isDarkMode = ref(storedTheme === null ? false : storedTheme === 'true');

  const applyTheme = () => {
    const html = document.documentElement;
    if (isDarkMode.value) {
      html.setAttribute('data-theme', 'dark');
    } else {
      html.setAttribute('data-theme', 'light');
    }
  };

  const toggleTheme = () => {
    isDarkMode.value = !isDarkMode.value;
  };

  watch(isDarkMode, (newValue) => {
    localStorage.setItem('darkMode', newValue);
    applyTheme();
  });

  onMounted(() => {
    applyTheme();
  });

  return {
    isDarkMode,
    toggleTheme
  };
}
