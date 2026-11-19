import orange from "../assets/emoji/orange.png";
import sparkles from "../assets/emoji/sparkles.png";
import rocket from "../assets/emoji/rocket.png";
import fire from "../assets/emoji/fire.png";
import game from "../assets/emoji/game.png";
import pkg from "../assets/emoji/package.png";
import robot from "../assets/emoji/robot.png";
import gear from "../assets/emoji/gear.png";
import users from "../assets/emoji/users.png";
import play from "../assets/emoji/play.png";
import stop from "../assets/emoji/stop.png";
import warning from "../assets/emoji/warning.png";
import check from "../assets/emoji/check.png";
import cross from "../assets/emoji/cross.png";
import wave from "../assets/emoji/wave.png";
import heart from "../assets/emoji/heart.png";
import star from "../assets/emoji/star.png";
import zap from "../assets/emoji/zap.png";
import folder from "../assets/emoji/folder.png";
import search from "../assets/emoji/search.png";
import download from "../assets/emoji/download.png";
import trash from "../assets/emoji/trash.png";
import wrench from "../assets/emoji/wrench.png";
import books from "../assets/emoji/books.png";
import trophy from "../assets/emoji/trophy.png";
import eyes from "../assets/emoji/eyes.png";
import bulb from "../assets/emoji/bulb.png";
import shield from "../assets/emoji/shield.png";
import key from "../assets/emoji/key.png";
import skull from "../assets/emoji/skull.png";
import party from "../assets/emoji/party.png";
import thinking from "../assets/emoji/thinking.png";
import sleep from "../assets/emoji/sleep.png";
import gem from "../assets/emoji/gem.png";
import cake from "../assets/emoji/cake.png";
import sun from "../assets/emoji/sun.png";
import leaf from "../assets/emoji/leaf.png";

export const emoji = {
  orange, sparkles, rocket, fire, game, package: pkg, robot, gear, users,
  play, stop, warning, check, cross, wave, heart, star, zap, folder, search,
  download, trash, wrench, books, trophy, eyes, bulb, shield, key, skull,
  party, thinking, sleep, gem, cake, sun, leaf,
} as const;

export type EmojiName = keyof typeof emoji;

export default function Emoji({
  name,
  size = "md",
  className = "",
}: {
  name: EmojiName;
  size?: "sm" | "md" | "lg" | "xl";
  className?: string;
}) {
  const cls = size === "lg" ? "emoji emoji-lg" : size === "xl" ? "emoji emoji-xl" : "emoji";
  return <img src={emoji[name]} alt="" aria-hidden className={`${cls} ${className}`} draggable={false} />;
}