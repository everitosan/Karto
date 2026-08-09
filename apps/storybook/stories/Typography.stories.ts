import type { Meta, StoryObj } from "@storybook/svelte";
import { Typography } from "@karto/ui";

const meta = {
  title: "UI/Typography",
  component: Typography,
  argTypes: {
    variant: {
      control: "select",
      options: [
        "display",
        "h1",
        "h2",
        "h3",
        "title",
        "subtitle",
        "body",
        "body-sm",
        "caption",
        "label",
      ],
    },
    color: { control: "inline-radio", options: ["default", "muted", "accent"] },
    align: { control: "inline-radio", options: ["left", "center", "right"] },
  },
} satisfies Meta<Typography>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Display: Story = {
  args: { variant: "display", children: "Mapea tu infraestructura" },
};

export const Heading: Story = {
  args: { variant: "h2", children: "Producción · Capa web" },
};

export const Body: Story = {
  args: { variant: "body", children: "Texto en Ubuntu Sans para contenido general." },
};

export const Accent: Story = {
  args: { variant: "label", color: "accent", children: "Conectar por SSH" },
};
