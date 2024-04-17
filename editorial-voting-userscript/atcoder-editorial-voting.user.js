// ==UserScript==
// @name         AtCoder Editorial Voting
// @namespace    https://atcoder.jp/
// @version      2024-04-17
// @description  AtCoderの解説に投票します。
// @author       magurofly
// @match        https://atcoder.jp/contests/*/editorial
// @match        https://atcoder.jp/contests/*/editorial?*
// @match        https://atcoder.jp/contests/*/tasks/*/editorial
// @match        https://atcoder.jp/contests/*/tasks/*/editorial?*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=atcoder.jp
// @grant        unsafeWindow
// @grant        GM_getValue
// @grant        GM_setValue
// ==/UserScript==

// AtCoder で定義されている以下の変数を使用します
// - contestScreenName
// - userScreenName
(function() {
    "use strict";

    // このスクリプトの機能
    // - 解説リンクに投票スコアと投票ボタンを表示する
    // - バックエンドにログインする（ため、一時的に所属欄を書き換える）
    // - 投票する

    let token = GM_getValue("token", null);

    function canonicalizeEditorialLink(url) {
        const prefix = "https://atcoder.jp/jump?url=";
        if (url.startsWith(prefix)) {
            return decodeURIComponent(url.slice(prefix.length));
        }
        return url;
    }

    function encodeFormData(data) {
        return Object.keys(data).map(key => encodeURIComponent(key) + "=" + encodeURIComponent(data[key]) ).join("&");
    }

    async function callApi(name, body) {
        return await fetch("https://magurofly.zapto.org/" + name, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(body),
        }).then(res => res.json());
    }

    async function login() {
        // 所属トークンを得る
        const affiliationTokenData = await callApi("create-affiliation-token", { atcoder_id: unsafeWindow.userScreenName });
        if (affiliationTokenData.status == "error") {
            throw data.reason;
        }
        const affiliation_token = affiliationTokenData.affiliation_token;

        // 設定を得る
        const profileSettings = new DOMParser().parseFromString(await fetch("https://atcoder.jp/settings").then(res => res.text()), "text/html");
        const data = {};
        for (const input of profileSettings.querySelector("#main-container form").elements) {
            data[input.name] = input.value;
        }
        const oldAffiliation = data["ui.Affiliation"];

        // 所属に所属トークンを設定する
        data["ui.Affiliation"] = affiliation_token;
        await fetch("https://atcoder.jp/settings", {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
            body: encodeFormData(data),
        });

        // 認証する
        const tokenData = await callApi("create-token", { atcoder_id: unsafeWindow.userScreenName, affiliation_token });
        if (tokenData.status == "error") {
            throw data.reason;
        }

        // 所属を元に戻す
        data["ui.Affiliation"] = oldAffiliation;
        await fetch("https://atcoder.jp/settings", {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
            body: encodeFormData(data),
        });

        // トークンを保存する
        token = tokenData.token;
        GM_setValue("token", token);
    }

    // 投票する
    async function sendVote(editorial, vote) {
        if (token == null) {
            await login();
        }

        callApi("vote", {
            token,
            contest: unsafeWindow.contestScreenName,
            editorial,
            vote,
        });
    }

    // 解説リンクにスコアと投票ボタンを表示する
    class Voting {
        constructor(editorial, elements) {
            this.editorial = editorial;
            this.elements = elements;
            this.score = 0;
            this.vote = 0;

            elements.btnUpVote.onclick = this.setVote.bind(this, 1);
            elements.btnDownVote.onclick = this.setVote.bind(this, -1);

            this.setVote(0, false);
            this.getVote();
        }

        async getVote() {
            callApi("status", { editorial: this.editorial, token }).then(data => {
                if (data.status == "error") {
                    console.error("AtCoderEditorialVoting: Error: " + data.reason);
                    return;
                }
                this.score = data.score;
                this.elements.scoreView.textContent = this.score;
                if (data.current_vote) {
                    switch (data.current_vote) {
                        case "up": this.vote = 1; this.setVote(1, false); break;
                        case "down": this.vote = -1; this.setVote(-1, false); break;
                        default: this.vote = 0; this.setVote(0, false);
                    }
                }
            });
        }

        async setVote(vote, send = true) {
            this.score -= this.vote;
            this.vote = vote;
            this.score += vote;
            if (vote == 1) {
                this.elements.btnUpVote.classList.add("active");
                this.elements.btnUpVote.onclick = this.setVote.bind(this, 0);
                this.elements.btnDownVote.classList.remove("active");
                this.elements.btnDownVote.onclick = this.setVote.bind(this, -1);
                if (send) await sendVote(this.editorial, "up");
            } else if (vote == -1) {
                this.elements.btnUpVote.classList.remove("active");
                this.elements.btnUpVote.onclick = this.setVote.bind(this, 1);
                this.elements.btnDownVote.classList.add("active");
                this.elements.btnDownVote.onclick = this.setVote.bind(this, 0);
                if (send) await sendVote(this.editorial, "down");
            } else {
                this.elements.btnUpVote.classList.remove("active");
                this.elements.btnUpVote.onclick = this.setVote.bind(this, 1);
                this.elements.btnDownVote.classList.remove("active");
                this.elements.btnDownVote.onclick = this.setVote.bind(this, -1);
                if (send) await sendVote(this.editorial, "none");
            }
            this.elements.scoreView.textContent = this.score;
        }
    }

    const votes = [];
    for (const link of unsafeWindow.document.querySelectorAll("#main-container a[rel=noopener]")) {
        // リンク先を正規化する
        const editorial = canonicalizeEditorialLink(link.href);

        const scoreView = document.createElement("span");
        scoreView.className = "label label-default";
        scoreView.textContent = "0";

        const btnUpVote = document.createElement("button");
        btnUpVote.className = "btn btn-xs btn-success";
        btnUpVote.type = "button";
        btnUpVote.textContent = "+";

        const btnDownVote = document.createElement("button");
        btnDownVote.className = "btn btn-xs btn-warning";
        btnDownVote.type = "button";
        btnDownVote.textContent = "-";

        const buttonGroup = document.createElement("span");
        buttonGroup.appendChild(btnDownVote);
        buttonGroup.appendChild(scoreView);
        buttonGroup.appendChild(btnUpVote);
        link.parentElement.insertBefore(buttonGroup, link);

        votes.push(new Voting(editorial, { scoreView, btnUpVote, btnDownVote, buttonGroup }));
    }
})();