import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

type LibraryContextValue = {
  epoch: number;
  bump: () => void;
};

const LibraryContext = createContext<LibraryContextValue>({
  epoch: 0,
  bump: () => {},
});

export function LibraryProvider({ children }: { children: ReactNode }) {
  const [epoch, setEpoch] = useState(0);
  const bump = useCallback(() => setEpoch((n) => n + 1), []);
  const value = useMemo(() => ({ epoch, bump }), [epoch, bump]);
  return <LibraryContext.Provider value={value}>{children}</LibraryContext.Provider>;
}

export function useLibrary() {
  return useContext(LibraryContext);
}
