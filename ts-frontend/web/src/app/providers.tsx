"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { WagmiProvider, createConfig, http } from "wagmi";
import { sepolia } from "wagmi/chains";
import { mock } from "wagmi/connectors";
import { ReactNode, useState } from "react";

const config = createConfig({
  chains: [sepolia],
  connectors: [
    mock({
      accounts: ["0x0000000000000000000000000000000000000001"],
    }),
  ],
  multiInjectedProviderDiscovery: false,
  transports: {
    [sepolia.id]: http("https://ethereum-sepolia-rpc.publicnode.com"),
  },
});

export function Providers({ children }: { children: ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <WagmiProvider config={config}>
        {children}
      </WagmiProvider>
    </QueryClientProvider>
  );
}
