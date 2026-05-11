/* @refresh reload */
import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import App from "./App";
import { ServersPage } from "./pages/ServersPage";
import { LeaguePage } from "./pages/LeaguePage";
import { TasksPage } from "./pages/TasksPage";
import { ProfilePage } from "./pages/ProfilePage";
import { SettingsPage } from "./pages/SettingsPage";
import DiscoverPage from "./pages/DiscoverPage";
import NewsPage from "./pages/NewsPage";
import { LaunchPage } from "./pages/LaunchPage";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
    },
  },
});

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element not found");
}

render(
  () => (
    <QueryClientProvider client={queryClient}>
      <Router root={App}>
        <Route path="/" component={ServersPage} />
        <Route path="/league" component={LeaguePage} />
        <Route path="/discover" component={DiscoverPage} />
        <Route path="/tasks" component={TasksPage} />
        <Route path="/news" component={NewsPage} />
        <Route path="/launch" component={LaunchPage} />
        <Route path="/profile" component={ProfilePage} />
        <Route path="/settings" component={SettingsPage} />
      </Router>
    </QueryClientProvider>
  ),
  root,
);
