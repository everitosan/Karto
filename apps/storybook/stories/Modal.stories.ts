import type { Meta, StoryObj } from "@storybook/svelte";
import { Modal } from "@karto/ui";

const meta = {
  title: "UI/Modal",
  component: Modal,
  argTypes: {
    open: { control: "boolean" },
    title: { control: "text" },
    width: { control: "text" },
  },
} satisfies Meta<Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    open: true,
    title: "Agregar credencial",
    onClose: () => {},
    children: "Contenido del modal (formulario, texto, etc.).",
  },
};
