import {
  Gamepad2, Rocket, Wine, Disc3, Swords, Crosshair, Radar, Waves, Plug, Fish,
  Bug, Database, Search, AppWindow, KeyRound, FileText, Wifi, Eye, Unlock,
  Bomb, Binary, Map, Skull, Share2, Server, Telescope, Globe, Lock,
  Microscope, Brain, FolderOpen, Image, Link2, CircleDot, Code, Box, Gem,
  MessageCircle, Mail, Cloud, Monitor, HardDrive, Zap, Send, Moon,
  GitCompare, Terminal, PlayCircle, Music, Film, Scissors, Video, Camera,
  Headphones, Shield, PenTool, Palette, Ruler, Printer, BookOpen, Radio,
  Cpu, Wrench, Package as PackageIcon, Coffee, Fingerprint, Snowflake,
  Star, WifiOff, History, ListOrdered, RotateCcw, AlertTriangle,
} from "lucide-solid";
import type { JSX } from "solid-js";
import { Dynamic } from "solid-js/web";

/** Minimal shape shared by every lucide-solid icon component — avoids
 *  depending on a specific exported type name from the icon package. */
export type IconComponent = (props: {
  size?: number | string;
  strokeWidth?: number | string;
  color?: string;
  class?: string;
}) => JSX.Element;

export const ICONS: Record<string, IconComponent> = {
  Gamepad2, Rocket, Wine, Disc3, Swords, Crosshair, Radar, Waves, Plug, Fish,
  Bug, Database, Search, AppWindow, KeyRound, FileText, Wifi, Eye, Unlock,
  Bomb, Binary, Map, Skull, Share2, Server, Telescope, Globe, Lock,
  Microscope, Brain, FolderOpen, Image, Link2, CircleDot, Code, Box, Gem,
  MessageCircle, Mail, Cloud, Monitor, HardDrive, Zap, Send, Moon,
  GitCompare, Terminal, PlayCircle, Music, Film, Scissors, Video, Camera,
  Headphones, Shield, PenTool, Palette, Ruler, Printer, BookOpen, Radio,
  Cpu, Wrench, Coffee, Fingerprint,
};

/** Icons for the ad-hoc "Discover" search results, keyed by package source. */
export const SOURCE_ICONS: Record<string, IconComponent> = {
  apt: PackageIcon,
  flatpak: Box,
  snap: CircleDot,
  brew: Coffee,
  hpm: Server,
  nix: Snowflake,
  appimage: AppWindow,
};

/** A few extra icons used by the new UI (offline banner, history/queue
 *  views, star ratings) — kept in their own small export so the big
 *  `ICONS` map above (looked up dynamically by package name) doesn't grow
 *  fuzzy matches against unrelated UI chrome icons. */
export const UI_ICONS = { Star, WifiOff, History, ListOrdered, RotateCcw, AlertTriangle };

/** Renders a package icon by name, falling back to a generic package glyph
 *  if the name isn't in the map (keeps the UI from breaking on typos
 *  instead of silently showing nothing). */
export function PkgIcon(props: { name: string; size?: number }) {
  return <Dynamic component={ICONS[props.name] ?? PackageIcon} size={props.size ?? 18} strokeWidth={1.8} />;
}

export function SourceIcon(props: { source: string; size?: number }) {
  return <Dynamic component={SOURCE_ICONS[props.source] ?? PackageIcon} size={props.size ?? 16} strokeWidth={1.8} />;
}
