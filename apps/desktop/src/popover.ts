import { mount } from "svelte";
import "./app.css";
import Popover from "./Popover.svelte";

document.body.classList.add("popover");
export default mount(Popover, { target: document.getElementById("app")! });
