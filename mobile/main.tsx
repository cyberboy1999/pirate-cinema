import React from "react";
import {createRoot} from "react-dom/client";
import MobileApp from "../app/mobile/page";
import "./root.css";

createRoot(document.getElementById("root")!).render(<React.StrictMode><MobileApp/></React.StrictMode>);
