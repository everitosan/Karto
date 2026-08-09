import type { Meta, StoryObj } from "@storybook/svelte";
import { Icon, icons, nodeTypeIcon } from "@karto/ui";

const meta = {
  title: "UI/Icon",
  component: Icon,
  argTypes: {
    size: { control: { type: "range", min: 12, max: 64, step: 2 } },
    strokeWidth: { control: { type: "range", min: 1, max: 3, step: 0.1 } },
    color: { control: "color" },
  },
} satisfies Meta<Icon>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Folder: Story = {
  args: { icon: icons.folder, size: 32, color: "var(--karto-color-accent)" },
};

export const Diagram: Story = {
  args: { icon: icons.diagram, size: 32, color: "var(--karto-color-accent)" },
};

export const ServerNode: Story = {
  args: { icon: nodeTypeIcon.server, size: 32 },
};

export const Firewall: Story = {
  args: { icon: nodeTypeIcon.firewall, size: 32 },
};
