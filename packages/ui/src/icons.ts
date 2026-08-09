// Catálogo curado de iconos de HugeIcons (free) usados en Karto.
// Import selectivo: solo lo que usamos entra al bundle.
import {
  Folder01Icon,
  WorkflowSquare01Icon,
  ViewIcon,
  ViewOffSlashIcon,
  Copy01Icon,
  PlusSignIcon,
  Search01Icon,
  Cancel01Icon,
  SquareLock01Icon,
  Key01Icon,
  Logout01Icon,
  MoreHorizontalIcon,
  Delete02Icon,
  Edit02Icon,
  Link01Icon,
  ComputerTerminal01Icon,
  ConnectIcon,
  ArrowRight01Icon,
  LayoutLeftIcon,
} from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/svelte";

/** Iconos semánticos de UI. */
export const icons = {
  folder: Folder01Icon,
  diagram: WorkflowSquare01Icon,
  eye: ViewIcon,
  eyeOff: ViewOffSlashIcon,
  copy: Copy01Icon,
  add: PlusSignIcon,
  search: Search01Icon,
  close: Cancel01Icon,
  lock: SquareLock01Icon,
  key: Key01Icon,
  logout: Logout01Icon,
  more: MoreHorizontalIcon,
  delete: Delete02Icon,
  edit: Edit02Icon,
  link: Link01Icon,
  terminal: ComputerTerminal01Icon,
  connect: ConnectIcon,
  chevron: ArrowRight01Icon,
  sidebar: LayoutLeftIcon,
} satisfies Record<string, IconSvgElement>;
