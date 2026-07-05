import {
  Gamepad2, Rocket, Wine, Disc3, Swords, Crosshair, Radar, Waves, Plug, Fish,
  Bug, Database, Search, AppWindow, KeyRound, FileText, Wifi, Eye, Unlock,
  Bomb, Binary, Map, Skull, Share2, Server, Telescope, Globe, Lock,
  Microscope, Brain, FolderOpen, Image, Link2, CircleDot, Code, Box, Gem,
  MessageCircle, Mail, Cloud, Monitor, HardDrive, Zap, Send, Moon,
  GitCompare, Terminal, PlayCircle, Music, Film, Scissors, Video, Camera,
  Headphones, Shield, PenTool, Palette, Ruler, Printer, BookOpen, Radio,
  Cpu, Wrench, Package as PackageIcon, Coffee, Fingerprint,
  type LucideIcon,
} from "lucide-react";

export const ICONS: Record<string, LucideIcon> = {
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
export const SOURCE_ICONS: Record<string, LucideIcon> = {
  apt: PackageIcon,
  flatpak: Box,
  snap: CircleDot,
  brew: Coffee,
};

/** Renders a package icon by name, falling back to a generic package glyph
 *  if the name isn't in the map (keeps the UI from breaking on typos
 *  instead of silently showing nothing). */
export function PkgIcon({ name, size = 18 }: { name: string; size?: number }) {
  const Comp = ICONS[name] ?? PackageIcon;
  return <Comp size={size} strokeWidth={1.8} />;
}

export function SourceIcon({ source, size = 16 }: { source: string; size?: number }) {
  const Comp = SOURCE_ICONS[source] ?? PackageIcon;
  return <Comp size={size} strokeWidth={1.8} />;
}
