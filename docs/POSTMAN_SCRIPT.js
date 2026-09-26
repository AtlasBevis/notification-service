const CryptoJS = require('crypto-js');

const CLIENT_ID = "client_id";
const SECRET = "secrets";

const timestamp = Math.floor(Date.now() / 1000);
const method = pm.request.method.toUpperCase();
const path = pm.request.url.getPathWithQuery();

const data = {
    idcard: "006193003790",
    date: "20260731"
};
const body = JSON.stringify(data)

const methodBytes = CryptoJS.enc.Utf8.parse(method);
const pathBytes = CryptoJS.enc.Utf8.parse(path);
const timeBytes = CryptoJS.enc.Utf8.parse(timestamp);
const bodyBytes = CryptoJS.enc.Utf8.parse(body);

let msg = methodBytes
    .concat(pathBytes)
    .concat(timeBytes)
    .concat(bodyBytes);

const signature = CryptoJS.HmacSHA256(msg, secret)
    .toString(CryptoJS.enc.Hex);

const methodBytes = CryptoJS.enc.Utf8.parse(method);
const timestamp = Math.floor(Date.now() / 1000);
const method = pm.request.method.toUpperCase();
const path = "/" + pm.request.url.path.join("/");

let msg = methodBytes
    .concat(pathBytes)
    .concat(timeBytes)
    .concat(bodyBytes);

const signature = CryptoJS.HmacSHA256(msg, secret)
    .toString(CryptoJS.enc.Hex);

// ====== DEBUG
// const hexMsg = msg.toString(CryptoJS.enc.Hex);
// const rawString = msg.toString(CryptoJS.enc.Utf8);
// console.log("path", path)
// console.log("timestamp", timestamp)
// console.log("signature", signature)
// console.log("hexMsg", hexMsg)
// console.log("rawString", rawString)
// console.log("HEX SECRET:", CryptoJS.enc.Hex.stringify(CryptoJS.enc.Utf8.parse(secret)));

pm.request.headers.upsert({ key: "x-client-id", value: client_id });
pm.request.headers.upsert({ key: "x-timestamp", value: timestamp });
pm.request.headers.upsert({ key: "x-signature", value: signature });
pm.environment.set("body", body)