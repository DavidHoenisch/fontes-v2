"use client";
import { useState } from "react";
import { Button, Card, TextField, Label, Input } from "@heroui/react";
import { ArrowLeft } from "lucide-react";
import Image from "next/image";
import { authClient } from "@/lib/auth-client";
export function AuthForm() {
  const [signup, setSignup] = useState(false),
    [message, setMessage] = useState(""),
    [busy, setBusy] = useState(false),
    [verificationEmail, setVerificationEmail] = useState("");
  return (
    <main className="auth-page">
      <a href="/" className="text-link">
        <ArrowLeft size={16} /> Workspace
      </a>
      <Card className="auth-card">
        <Card.Content>
          <Image
            className="brand-mark"
            src="/brand/app-icon-rounded.svg"
            alt="Fontes"
            width={48}
            height={48}
          />
          <p className="eyebrow">FONTES PUBLISHING</p>
          <h1>{signup ? "Make room for your work." : "Welcome back."}</h1>
          <p className="muted">
            {signup
              ? "Create your publishing account. Ordinary offline study never requires one."
              : "Sign in to your publishing workspace."}
          </p>
          {verificationEmail ? (
            <form
              onSubmit={async (e) => {
                e.preventDefault();
                setBusy(true);
                setMessage("");
                const f = new FormData(e.currentTarget);
                try {
                  const result = await authClient.emailOtp.verifyEmail({
                    email: verificationEmail,
                    otp: String(f.get("otp")),
                  });
                  if (result.error)
                    setMessage(
                      result.error.message ?? "Check the code and try again.",
                    );
                  else window.location.assign("/");
                } catch {
                  setMessage("Verification is unavailable. Please try again.");
                } finally {
                  setBusy(false);
                }
              }}
            >
              <p className="muted">
                Enter the verification code sent to {verificationEmail}.
              </p>
              <TextField name="otp" isRequired>
                <Label>Email verification code</Label>
                <Input autoComplete="one-time-code" inputMode="numeric" />
              </TextField>
              {message && (
                <p role="status" className="form-message">
                  {message}
                </p>
              )}
              <Button type="submit" className="primary-button" isPending={busy}>
                Verify email
              </Button>
              <Button
                variant="ghost"
                isDisabled={busy}
                onPress={async () => {
                  setBusy(true);
                  try {
                    const result =
                      await authClient.emailOtp.sendVerificationOtp({
                        email: verificationEmail,
                        type: "email-verification",
                      });
                    setMessage(
                      result.error?.message ??
                        "A new code has been requested. Check your inbox.",
                    );
                  } catch {
                    setMessage("Unable to resend. Please retry.");
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                Resend code
              </Button>
              <Button variant="ghost" onPress={() => setVerificationEmail("")}>
                Back to sign in
              </Button>
            </form>
          ) : (
            <form
              onSubmit={async (e) => {
                e.preventDefault();
                setBusy(true);
                setMessage("");
                const f = new FormData(e.currentTarget);
                try {
                  const credentials = {
                    email: String(f.get("email")),
                    password: String(f.get("password")),
                  };
                  const result = signup
                    ? await authClient.signUp.email({
                        ...credentials,
                        name: String(f.get("name")),
                      })
                    : await authClient.signIn.email(credentials);
                  if (result.error) {
                    setMessage(
                      result.error.message ?? "Unable to authenticate.",
                    );
                    if (result.error.code === "EMAIL_NOT_VERIFIED")
                      setVerificationEmail(credentials.email);
                  } else if (signup) setVerificationEmail(credentials.email);
                  else window.location.assign("/");
                } catch {
                  setMessage(
                    "Authentication is unavailable. Please try again.",
                  );
                } finally {
                  setBusy(false);
                }
              }}
            >
              {signup && (
                <TextField name="name" isRequired>
                  <Label>Name</Label>
                  <Input autoComplete="name" />
                </TextField>
              )}
              <TextField name="email" type="email" isRequired>
                <Label>Email address</Label>
                <Input autoComplete="email" />
              </TextField>
              <TextField
                name="password"
                type="password"
                isRequired
                minLength={signup ? 12 : undefined}
              >
                <Label>Password</Label>
                <Input
                  autoComplete={signup ? "new-password" : "current-password"}
                />
              </TextField>
              {message && (
                <p role="status" className="form-message">
                  {message}
                </p>
              )}
              <Button type="submit" isPending={busy} className="primary-button">
                {signup ? "Create account" : "Sign in"}
              </Button>
              <Button
                variant="ghost"
                onPress={() => {
                  setSignup(!signup);
                  setMessage("");
                }}
              >
                {signup
                  ? "Already have an account? Sign in"
                  : "New to publishing? Create an account"}
              </Button>
            </form>
          )}
        </Card.Content>
      </Card>
      <p className="auth-footnote">
        In local development, open the workspace directly. No login is needed.
      </p>
    </main>
  );
}
