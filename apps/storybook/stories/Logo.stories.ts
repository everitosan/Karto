import type { Meta, StoryObj } from "@storybook/svelte";
import { Logo } from "@karto/ui";

const meta = {
  title: "Marca/Logo",
  component: Logo,
  argTypes: {
    variant: { control: "inline-radio", options: ["full", "iso"] },
    size: { control: { type: "range", min: 24, max: 160, step: 4 } },
    color: { control: "color" },
  },
  parameters: { backgrounds: { default: "karto" } },
} satisfies Meta<Logo>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Full: Story = {
  args: { variant: "full", size: 72 },
};

export const Iso: Story = {
  args: { variant: "iso", size: 96 },
};
