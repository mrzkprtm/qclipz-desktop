// One stroke icon set: 20px grid, 1.6 stroke, round caps.
import type { ReactNode } from "react";

function Icon({ children, size = 18 }: { children: ReactNode; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6"
      strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {children}
    </svg>
  );
}

type P = { size?: number };

export const IconScreen = (p: P) => (
  <Icon {...p}><rect x="2.5" y="3.5" width="15" height="10" rx="1.5" /><path d="M7 17h6M10 13.5V17" /></Icon>
);
export const IconArea = (p: P) => (
  <Icon {...p}><path d="M2.5 6V3.5H5M15 3.5h2.5V6M17.5 14v2.5H15M5 16.5H2.5V14" /><rect x="6" y="7" width="8" height="6" rx="1" /></Icon>
);
export const IconWindow = (p: P) => (
  <Icon {...p}><rect x="2.5" y="3.5" width="15" height="13" rx="1.5" /><path d="M2.5 7h15M5 5.25h.01M7 5.25h.01" /></Icon>
);
export const IconRefresh = (p: P) => (
  <Icon {...p}><path d="M16 10a6 6 0 1 1-1.8-4.3M16 3.5V6.5h-3" /></Icon>
);
export const IconFolder = (p: P) => (
  <Icon {...p}><path d="M2.5 5.5a1 1 0 0 1 1-1h4l1.5 2h7.5a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1z" /></Icon>
);
export const IconStop = (p: P) => (
  <Icon {...p}><rect x="5" y="5" width="10" height="10" rx="1.5" fill="currentColor" stroke="none" /></Icon>
);
export const IconCrop = (p: P) => (
  <Icon {...p}><path d="M5.5 2.5v11a1 1 0 0 0 1 1h11M2.5 5.5h11a1 1 0 0 1 1 1v11" /></Icon>
);
export const IconTrash = (p: P) => (
  <Icon {...p}><path d="M3.5 5.5h13M8 5.5V3.5h4v2M5 5.5l.8 11h8.4l.8-11M8.5 8.5v5M11.5 8.5v5" /></Icon>
);
export const IconPlay = (p: P) => (
  <Icon {...p}><path d="M6.5 4.5v11l9-5.5z" /></Icon>
);
export const IconCheck = (p: P) => (
  <Icon {...p}><path d="M4 10.5 8 14.5 16 6" /></Icon>
);
export const IconClose = (p: P) => (
  <Icon {...p}><path d="M5 5l10 10M15 5 5 15" /></Icon>
);
export const IconExpand = (p: P) => (
  <Icon {...p}><path d="M3 7.5V3h4.5M12.5 3H17v4.5M17 12.5V17h-4.5M7.5 17H3v-4.5" /></Icon>
);
export const IconMic = (p: P) => (
  <Icon {...p}><rect x="7.5" y="2.5" width="5" height="9" rx="2.5" /><path d="M4.5 9.5a5.5 5.5 0 0 0 11 0M10 15v2.5" /></Icon>
);
export const IconGrip = (p: P) => (
  <Icon {...p}><path d="M7.5 5h.01M12.5 5h.01M7.5 10h.01M12.5 10h.01M7.5 15h.01M12.5 15h.01" strokeWidth="2.4" /></Icon>
);
export const IconOpen = (p: P) => (
  <Icon {...p}><path d="M11 3.5h5.5V9M16.5 3.5 9 11M14 12v3.5a1 1 0 0 1-1 1H4.5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1H8" /></Icon>
);
export const IconSearch = (p: P) => (
  <Icon {...p}><circle cx="9" cy="9" r="5.5" /><path d="m13 13 4 4" /></Icon>
);
export const IconRecord = (p: P) => (
  <Icon {...p}><circle cx="10" cy="10" r="5" fill="currentColor" stroke="none" /></Icon>
);
export const IconRestore = (p: P) => (
  <Icon {...p}><rect x="2.5" y="6.5" width="11" height="10" rx="1.5" /><path d="M6.5 3.5h9.5a1 1 0 0 1 1 1V13" /></Icon>
);
export const IconSpeaker = (p: P) => (
  <Icon {...p}><path d="M3.5 7.5h3l4-3.5v12l-4-3.5h-3z" /><path d="M13.5 7.5a3.5 3.5 0 0 1 0 5M15.5 5a7 7 0 0 1 0 10" /></Icon>
);
