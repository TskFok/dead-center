import { SettingsApp } from "./SettingsApp";
import { tauriBridge } from "./shared/bridge";

export default function App() {
  return <SettingsApp bridge={tauriBridge} />;
}
