import type { Preview } from "@storybook/svelte";
import "@karto/ui/styles.css";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: { color: /(background|color)$/i, date: /Date$/i },
    },
    backgrounds: {
      default: "karto",
      values: [
        { name: "karto", value: "#090d15" },
        { name: "black", value: "#000000" },
        { name: "light", value: "#ffffff" },
      ],
    },
  },
};

export default preview;
