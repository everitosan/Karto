import type { Meta, StoryObj } from "@storybook/svelte";
import { Button } from "@karto/ui";

const meta = {
  title: "UI/Button",
  component: Button,
  argTypes: {
    variant: {
      control: "select",
      options: ["primary", "secondary", "ghost", "danger"],
    },
    disabled: { control: "boolean" },
  },
} satisfies Meta<Button>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: { variant: "primary", children: "Conectar por SSH" },
};

export const Secondary: Story = {
  args: { variant: "secondary", children: "Abrir vault" },
};

export const Danger: Story = {
  args: { variant: "danger", children: "Eliminar nodo" },
};
