const PATHS = {
  archive: "M4 7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7z",
  activity:
    "M3 12h4l3 8 4-16 3 8h4",
  settings:
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 13a7.5 7.5 0 0 0 0-2l2-1.5-2-3.4-2.3 1a7.6 7.6 0 0 0-1.7-1L15 3h-6l-.4 2.6a7.6 7.6 0 0 0-1.7 1l-2.3-1-2 3.4L4.6 11a7.5 7.5 0 0 0 0 2l-2 1.5 2 3.4 2.3-1a7.6 7.6 0 0 0 1.7 1L9 21h6l.4-2.6a7.6 7.6 0 0 0 1.7-1l2.3 1 2-3.4-2-1.5z",
  browser:
    "M12 3a9 9 0 1 0 9 9M3 12a9 9 0 0 1 9-9M3 12h18M12 3c2.5 2.6 3.8 5.7 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.7-3.8-9S9.5 5.6 12 3z",
  check: "M5 12.5l4.5 4.5L19 7.5",
  refresh:
    "M20 11a8 8 0 0 0-14.9-3M4 13a8 8 0 0 0 14.9 3M4 4v5h5M20 20v-5h-5",
  play: "M8 5v14l11-7z",
  stop: "M7 7h10v10H7z",
  download:
    "M12 4v11m0 0l-4.5-4.5M12 15l4.5-4.5M4 19h16",
  copy: "M9 9h10v11H9zM5 15V4h10",
  folder:
    "M4 7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7z",
  info: "M12 8h.01M12 11v6M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16z",
  file: "M6 3h8l4 4v14H6zM14 3v5h5M9 13h6M9 17h6",
};

export default function Icon({ name, size = 16 }) {
  const path = PATHS[name] || PATHS.info;
  const dimension = Number.isFinite(size) ? size : 16;
  return (
    <svg
      className="icon"
      width={dimension}
      height={dimension}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={path} />
    </svg>
  );
}
